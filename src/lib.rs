use std::collections::VecDeque;
use std::fmt::{Debug, Formatter};
use std::sync::atomic::{AtomicBool, AtomicUsize, Ordering};
use std::sync::{Arc, Condvar, Mutex};

/// A thread-safe, generic queue that can operate in either FIFO (First-In, First-Out)
/// or LIFO (Last-In, First-Out) mode.
///
/// The queue is implemented using a `VecDeque` protected by a `Mutex` for interior mutability,
/// making it safe to share across multiple threads. It also uses a `Condvar` to allow for
/// efficient blocking waits for items.
///
/// # Examples
///
/// ```
/// use kraquen::{Queue, QueueMode};
///
/// // Create a new FIFO queue
/// let fifo_queue = Queue::new(QueueMode::FIFO);
/// fifo_queue.push(1);
/// fifo_queue.push(2);
/// assert_eq!(fifo_queue.pop(), Some(1));
///
/// // Create a new LIFO queue
/// let lifo_queue = Queue::new(QueueMode::LIFO);
/// lifo_queue.push(1);
/// lifo_queue.push(2);
/// assert_eq!(lifo_queue.pop(), Some(2));
/// ```
#[derive(Clone)]
pub struct Queue<T> {
    shared: Arc<SharedState<T>>,
}

impl<T: Debug> Debug for Queue<T> {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        let guard = self.shared.inner.lock().unwrap();
        match guard.mode {
            QueueMode::FIFO => f.debug_list().entries(guard.data.iter().rev()).finish(),
            QueueMode::LIFO => f.debug_list().entries(guard.data.iter()).finish(),
        }
    }
}

impl<T> Queue<T> {
    /// Creates a new queue with the specified mode.
    ///
    /// # Arguments
    ///
    /// * `mode` - The `QueueMode` (FIFO or LIFO) that determines the queue's behavior.
    ///
    pub fn new(mode: QueueMode) -> Queue<T> {
        let state = SharedState::new(mode, None);
        Queue {
            shared: Arc::new(state),
        }
    }
    /// Creates a new queue with the specified mode and a maximum capacity.
    ///
    /// This creates a bounded queue. If the queue reaches the specified `capacity`,
    /// pushing new items will not block. Instead, it will forcefully evict the
    /// next scheduled item (based on the `QueueMode`) to make room, and return it.
    ///
    /// * In `FIFO` mode, pushing to a full queue evicts the **oldest** item.
    /// * In `LIFO` mode, pushing to a full queue evicts the **newest** item.
    ///
    /// # Arguments
    ///
    /// * `mode` - The `QueueMode` (FIFO or LIFO) that determines the queue's behavior.
    /// * `capacity` - The maximum number of items the queue can hold.
    ///
    /// # Examples
    ///
    /// ```
    /// use kraquen::{Queue, QueueMode};
    ///
    /// // Create a bounded FIFO queue that holds a maximum of 2 items
    /// let queue = Queue::with_capacity(QueueMode::FIFO, 2);
    ///
    /// assert_eq!(queue.push(10), None); // Plenty of room
    /// assert_eq!(queue.push(20), None); // Queue is now full: [10, 20]
    ///
    /// // Pushing a 3rd item evicts the oldest item (10)
    /// let evicted = queue.push(30);
    /// assert_eq!(evicted, Some(10));
    ///
    /// // The queue now contains [20, 30]
    /// assert_eq!(queue.pop(), Some(20));
    /// ```
    pub fn with_capacity(mode: QueueMode, capacity: usize) -> Queue<T> {
        let state = SharedState::new(mode, Some(capacity));
        Queue {
            shared: Arc::new(state),
        }
    }

    /// Initiates a shutdown of the queue.
    ///
    /// This sets the shutdown flag to true and notifies all threads that are currently
    /// waiting on the condvar in `pop_blocking` or `pop_timeout`. Once shutdown,
    /// no more items can be pushed to the queue.
    pub fn shutdown(&self) {
        self.shared
            .shutdown
            .store(true, std::sync::atomic::Ordering::SeqCst);
        self.shared.condvar.notify_all();
    }

    /// Tries to push an item onto the queue.
    ///
    /// If the queue has been shut down, this method will fail and return the item
    /// it was trying to push.
    ///
    /// # Arguments
    ///
    /// * `item` - The item to push onto the queue.
    ///
    /// # Returns
    ///
    /// * `Ok(Option<T>)` - If the item was successfully pushed. Optionally returns an evicted item T for bound capacity queues
    /// * `Err(T)` - If the queue is shut down, containing the item that was not pushed.
    pub fn try_push(&self, item: T) -> Result<Option<T>, T> {
        if !self
            .shared
            .shutdown
            .load(std::sync::atomic::Ordering::SeqCst)
        {
            let mut guard = self.shared.inner.lock().unwrap();
            let evicted = guard.push(item);
            self.shared.condvar.notify_one();
            self.shared
                .telemetry
                .total_pushed
                .fetch_add(1, Ordering::Relaxed);
            if evicted.is_some() {
                self.shared
                    .telemetry
                    .total_evicted
                    .fetch_add(1, Ordering::Relaxed);
            }
            return Ok(evicted);
        }
        Err(item)
    }

    /// Pushes an item onto the queue.
    ///
    /// This method will panic if the queue has been shut down. For a non-panicking
    /// version, see `try_push`.
    ///
    /// # Arguments
    ///
    /// * `item` - The item to push onto the queue.
    ///
    /// # Panics
    ///
    /// Panics if the queue is shut down.
    /// # Returns
    ///
    /// * `Option<T>` - An evicted item T for bound capacity queues
    pub fn push(&self, item: T) -> Option<T> {
        match self.try_push(item) {
            Ok(evicted) => evicted,
            Err(_) => panic!("Tried to push to a shut down queue"),
        }
    }

    /// Removes and returns an item from the queue.
    ///
    /// This is a non-blocking operation. The item is removed according to the queue's
    /// `QueueMode` (FIFO or LIFO).
    ///
    /// # Returns
    ///
    /// * `Some(T)` - If there was an item to pop.
    /// * `None` - If the queue was empty.
    pub fn pop(&self) -> Option<T> {
        let mut guard = self.shared.inner.lock().unwrap();
        let ret = guard.pop();
        if ret.is_some() {
            self.shared
                .telemetry
                .total_popped
                .fetch_add(1, Ordering::Relaxed);
        }
        ret
    }

    /// Removes and returns an item from the queue, blocking until an item is available.
    ///
    /// This method will block the current thread until an item is pushed to the queue
    /// or the queue is shut down.
    ///
    /// # Returns
    ///
    /// * `Some(T)` - If an item was successfully popped.
    /// * `None` - If the queue is empty and has been shut down.
    pub fn pop_blocking(&self) -> Option<T> {
        self.shared
            .telemetry
            .waiting_consumers
            .fetch_add(1, Ordering::Relaxed);
        let guard = self.shared.inner.lock().unwrap();
        let mut guard = self
            .shared
            .condvar
            .wait_while(guard, |inner| {
                inner.data.is_empty()
                    && !self
                        .shared
                        .shutdown
                        .load(std::sync::atomic::Ordering::SeqCst)
            })
            .unwrap();
        self.shared
            .telemetry
            .waiting_consumers
            .fetch_sub(1, Ordering::Relaxed);
        let ret = guard.pop();
        if ret.is_some() {
            self.shared
                .telemetry
                .total_popped
                .fetch_add(1, Ordering::Relaxed);
        }
        ret
    }

    /// Removes and returns an item from the queue, blocking until an item is available or a timeout is reached.
    ///
    /// This method will block the current thread until an item is pushed, the timeout
    /// elapses, or the queue is shut down.
    ///
    /// # Arguments
    ///
    /// * `timeout` - The maximum `Duration` to wait for an item.
    ///
    /// # Returns
    ///
    /// * `Some(T)` - If an item was successfully popped.
    /// * `None` - If the timeout was reached or if the queue is empty and has been shut down.
    pub fn pop_timeout(&self, timeout: std::time::Duration) -> Option<T> {
        self.shared
            .telemetry
            .waiting_consumers
            .fetch_add(1, Ordering::Relaxed);
        let guard = self.shared.inner.lock().unwrap();
        let (mut guard, _) = self
            .shared
            .condvar
            .wait_timeout_while(guard, timeout, |queue| {
                queue.data.is_empty()
                    && !self
                        .shared
                        .shutdown
                        .load(std::sync::atomic::Ordering::SeqCst)
            })
            .unwrap();
        self.shared
            .telemetry
            .waiting_consumers
            .fetch_sub(1, Ordering::Relaxed);
        let ret = guard.pop();
        if ret.is_some() {
            self.shared
                .telemetry
                .total_popped
                .fetch_add(1, Ordering::Relaxed);
        }
        ret
    }

    /// Returns the number of items currently in the queue.
    pub fn len(&self) -> usize {
        let guard = self.shared.inner.lock().unwrap();
        guard.data.len()
    }

    /// Returns `true` if the queue contains no items.
    pub fn is_empty(&self) -> bool {
        let guard = self.shared.inner.lock().unwrap();
        guard.data.is_empty()
    }

    /// Peeks at the next item in the queue without removing it.
    ///
    /// Applies a closure to a reference to the item that would be returned by `pop`.
    ///
    /// # Returns
    ///
    /// * `Some(R)` - If the queue is not empty, containing the return value of the closure.
    /// * `None` - If the queue is empty.
    pub fn peek<R, F>(&self, f: F) -> Option<R>
    where
        F: FnOnce(&T) -> R,
    {
        let guard = self.shared.inner.lock().unwrap();
        guard.peek().map(f)
    }

    /// Removes all items from the queue.
    pub fn clear(&self) {
        let mut guard = self.shared.inner.lock().unwrap();
        guard.data.clear();
    }

    /// Returns a snapshot of the current queue metrics.
    ///
    /// This provides insight into the queue's health, including total traffic,
    /// eviction counts, and the number of currently waiting consumers.
    pub fn snapshot(&self) -> QueueSnapshot {
        let tel = &self.shared.telemetry;
        QueueSnapshot {
            current_len: self.len(),
            total_pushed: tel.total_pushed.load(Ordering::Relaxed),
            total_popped: tel.total_popped.load(Ordering::Relaxed),
            total_evicted: tel.total_evicted.load(Ordering::Relaxed),
            waiting_consumers: tel.waiting_consumers.load(Ordering::Relaxed),
        }
    }
}
/// Specifies the operational mode of the queue.
pub enum QueueMode {
    /// First-In, First-Out. Items are removed in the same order they are added.
    FIFO,
    /// Last-In, First-Out. The most recently added item is the first one to be removed.
    LIFO,
}
struct InnerQueue<T> {
    mode: QueueMode,
    data: VecDeque<T>,
    capacity: Option<usize>,
}

impl<T> InnerQueue<T> {
    fn new(mode: QueueMode, capacity: Option<usize>) -> InnerQueue<T> {
        InnerQueue {
            mode,
            data: VecDeque::new(),
            capacity,
        }
    }
    pub fn push(&mut self, item: T) -> Option<T> {
        let evicted = match self.capacity {
            Some(cap) if self.data.len() >= cap => self.pop(),
            _ => None,
        };
        self.data.push_front(item);
        evicted
    }

    pub fn pop(&mut self) -> Option<T> {
        match self.mode {
            QueueMode::FIFO => self.data.pop_back(),
            QueueMode::LIFO => self.data.pop_front(),
        }
    }

    pub fn peek(&self) -> Option<&T> {
        match self.mode {
            QueueMode::FIFO => self.data.back(),
            QueueMode::LIFO => self.data.front(),
        }
    }
}

struct SharedState<T> {
    inner: Mutex<InnerQueue<T>>,
    condvar: Condvar,
    shutdown: AtomicBool,
    telemetry: Telemetry,
}

impl<T> SharedState<T> {
    pub fn new(mode: QueueMode, capacity: Option<usize>) -> Self {
        SharedState {
            inner: Mutex::new(InnerQueue::new(mode, capacity)),
            condvar: Condvar::new(),
            shutdown: AtomicBool::new(false),
            telemetry: Telemetry::new(),
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct QueueSnapshot {
    pub current_len: usize,
    pub total_pushed: usize,
    pub total_popped: usize,
    pub total_evicted: usize,
    pub waiting_consumers: usize,
}

struct Telemetry {
    total_pushed: AtomicUsize,
    total_popped: AtomicUsize,
    total_evicted: AtomicUsize,
    waiting_consumers: AtomicUsize,
}

impl Telemetry {
    fn new() -> Self {
        Telemetry {
            total_pushed: AtomicUsize::new(0),
            total_popped: AtomicUsize::new(0),
            total_evicted: AtomicUsize::new(0),
            waiting_consumers: AtomicUsize::new(0),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_push() {
        let queue: Queue<usize> = Queue::new(QueueMode::FIFO);
        queue.push(1);
        let element = queue.pop();
        assert_eq!(1, element.unwrap());
        assert_eq!(None, queue.pop());
    }
    #[test]
    fn test_fifo() {
        let queue: Queue<usize> = Queue::new(QueueMode::FIFO);
        queue.push(1);
        queue.push(2);
        queue.push(3);
        assert_eq!(1, queue.pop().unwrap());
        assert_eq!(2, queue.pop().unwrap());
        assert_eq!(3, queue.pop().unwrap());
    }

    #[test]
    fn test_lifo() {
        let queue: Queue<usize> = Queue::new(QueueMode::LIFO);
        queue.push(1);
        queue.push(2);
        queue.push(3);
        assert_eq!(3, queue.pop().unwrap());
        assert_eq!(2, queue.pop().unwrap());
        assert_eq!(1, queue.pop().unwrap());
    }
    #[test]
    fn test_peek_and_len() {
        let lifo: Queue<usize> = Queue::new(QueueMode::LIFO);
        lifo.push(10);

        let value = lifo.peek(|v| *v).unwrap();
        assert_eq!(value, 10);

        let fifo: Queue<usize> = Queue::new(QueueMode::FIFO);
        fifo.push(5);

        let fifo_value = fifo.peek(|v| *v).unwrap();
        assert_eq!(fifo_value, 5);
    }

    #[test]
    fn test_peek_ordering_and_clear() {
        let lifo: Queue<usize> = Queue::new(QueueMode::LIFO);
        lifo.push(1);
        lifo.push(2);
        lifo.push(3);

        assert_eq!(3, lifo.peek(|v| *v).unwrap());

        lifo.clear();
        assert_eq!(0, lifo.len());

        let fifo: Queue<usize> = Queue::new(QueueMode::FIFO);
        fifo.push(1);
        fifo.push(2);
        fifo.push(3);

        assert_eq!(1, fifo.peek(|v| *v).unwrap());

        fifo.clear();
        assert_eq!(0, fifo.len());
    }

    use std::thread;
    use std::time::Duration;

    #[test]
    fn test_multithreaded_concurrent_access() {
        // mutex provides interior mutability
        let queue = Queue::new(QueueMode::FIFO);
        let mut handles = vec![];

        for i in 0..10 {
            // Clone the handle, cheap
            let queue_clone = queue.clone();

            handles.push(thread::spawn(move || {
                for j in 0..100 {
                    queue_clone.push((i * 100) + j);
                }
            }));
        }

        // Wait for all threads to finish pushing
        for handle in handles {
            handle.join().unwrap();
        }

        // If the queue is thread-safe, all 1000 items (10 threads * 100 items) must be there.
        assert_eq!(queue.len(), 1000);
        assert!(!queue.is_empty());
    }

    #[test]
    fn test_pop_blocking() {
        let queue = Queue::new(QueueMode::FIFO);
        let queue_clone = queue.clone();
        let handle = thread::spawn(move || queue_clone.pop_blocking());

        thread::sleep(Duration::from_millis(100));
        queue.push(5);

        let value = handle.join().unwrap();
        assert_eq!(value, Some(5));
    }
    #[test]
    fn test_pop_timeout() {
        let queue: Queue<i32> = Queue::new(QueueMode::FIFO);

        let start = std::time::Instant::now();
        let result = queue.pop_timeout(Duration::from_millis(100));
        let elapsed = start.elapsed();

        assert_eq!(result, None);
        assert!(elapsed >= Duration::from_millis(100));

        queue.push(42);
        let result = queue.pop_timeout(Duration::from_millis(100));
        assert_eq!(result, Some(42));
    }
    #[test]
    fn test_debug_print() {
        let fifo = Queue::new(QueueMode::FIFO);
        fifo.push(10);
        fifo.push(20);

        let output = format!("{:?}", fifo);

        assert_eq!(output, "[10, 20]");

        let lifo = Queue::new(QueueMode::LIFO);
        lifo.push(10);
        lifo.push(20);

        let output = format!("{:?}", lifo);

        assert_eq!(output, "[20, 10]");
    }

    #[test]
    fn test_non_clone_types() {
        // A struct that explicitly cannot be cloned
        #[derive(Debug, PartialEq)]
        struct UniqueItem {
            id: i32,
        }

        let queue = Queue::new(QueueMode::FIFO);
        queue.push(UniqueItem { id: 1 });
        let item = queue.pop().unwrap();

        assert_eq!(item, UniqueItem { id: 1 });
    }

    #[test]
    fn test_peek_on_empty() {
        let queue: Queue<i32> = Queue::new(QueueMode::FIFO);

        let result = queue.peek(|v| *v);

        assert_eq!(result, None);
    }

    #[test]
    fn test_bounded_capacity_eviction_fifo() {
        let queue = Queue::with_capacity(QueueMode::FIFO, 2);

        // Push first two items (plenty of room, returns None)
        assert_eq!(queue.push(1), None);
        assert_eq!(queue.push(2), None);

        // Queue is now full [1, 2].
        // Pushing a 3rd item should evict the oldest item (1)
        let evicted = queue.push(3);
        assert_eq!(evicted, Some(1));

        // Queue should now contain [2, 3]
        assert_eq!(queue.len(), 2);
        assert_eq!(queue.pop(), Some(2));
        assert_eq!(queue.pop(), Some(3));
    }

    #[test]
    fn test_bounded_capacity_eviction_lifo() {
        let queue = Queue::with_capacity(QueueMode::LIFO, 2);

        assert_eq!(queue.push(1), None);
        assert_eq!(queue.push(2), None);

        // Queue is LIFO, so the "newest" item at the front is 2.
        // Pushing 3 should evict the 2!
        let evicted = queue.push(3);
        assert_eq!(evicted, Some(2));

        // Queue should now contain [3, 1]
        assert_eq!(queue.len(), 2);
        assert_eq!(queue.pop(), Some(3));
        assert_eq!(queue.pop(), Some(1));
    }

    #[test]
    fn test_telemetry_snapshot() {
        let queue = Queue::with_capacity(QueueMode::FIFO, 2);

        queue.push(1);
        queue.push(2);
        queue.push(3); // This triggers an eviction
        queue.pop(); // One successful pop

        let stats = queue.snapshot();

        assert_eq!(stats.total_pushed, 3);
        assert_eq!(stats.total_evicted, 1);
        assert_eq!(stats.total_popped, 1);
        assert_eq!(stats.current_len, 1);
        assert_eq!(stats.waiting_consumers, 0);
    }
}

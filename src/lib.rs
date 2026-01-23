use std::collections::VecDeque;
use std::fmt::{Debug, Formatter, Result};
use std::sync::{Arc, Condvar, Mutex};

#[derive(Clone)]
struct Queue<T> {
    queue: Arc<(Mutex<InnerQueue<T>>, Condvar)>
}

impl<T: Debug> Debug for Queue<T> {
    fn fmt(&self, f: &mut Formatter<'_>) -> Result {
        let guard = self.queue.0.lock().unwrap();
        match guard.mode {
            QueueMode::FIFO => {
                f.debug_list().entries(guard.data.iter().rev()).finish()
            }
            QueueMode::LIFO => {
                f.debug_list().entries(guard.data.iter()).finish()
            }
        }
    }
}

impl<T> Queue<T> {
    pub fn new(mode: QueueMode) -> Queue<T> {
        let queue = InnerQueue::new(mode);
        let x = Mutex::new(queue);
        let c = Condvar::new();
        Queue {
            queue: Arc::new((x, c))
        }
    }
    
    pub fn push(&self, item : T) {
        let (lock, cond) = &*self.queue;
        let mut guard = lock.lock().unwrap();
        guard.push(item);
        cond.notify_one();
    }

    pub fn pop(&self) -> Option<T> {
        let mut guard = self.queue.0.lock().unwrap();
        guard.pop()
    }

    pub fn pop_blocking(&self) -> T {
        let (lock, cond) = &*self.queue;
        let guard = lock.lock().unwrap();
        let mut guard = cond
            .wait_while(guard, |inner| inner.data.is_empty())
            .unwrap();
        guard.pop().unwrap()
    }


    pub fn pop_timeout(&self, timeout: std::time::Duration) -> Option<T> {
        let (lock, cvar) = &*self.queue;
        let guard = lock.lock().unwrap();

        let (mut guard, _) = cvar
            .wait_timeout_while(guard, timeout, |inner| inner.data.is_empty())
            .unwrap();

        guard.pop()
    }
    
    pub fn len(&self) -> usize {
        let guard = self.queue.0.lock().unwrap();
        guard.data.len()
    }

    pub fn is_empty(&self) -> bool {
        let guard = self.queue.0.lock().unwrap();
        guard.data.is_empty()
    }
    
    pub fn peek<R, F>(&self, f: F) -> Option<R> 
    where F: FnOnce(&T) -> R,
    {
        let guard = self.queue.0.lock().unwrap();
        guard.peek().map(f)
    }

    
    pub fn clear(&self) {
        let mut guard = self.queue.0.lock().unwrap();
        guard.data.clear();
    }
}
pub enum QueueMode {
    FIFO,
    LIFO
}
struct InnerQueue<T> {
    mode: QueueMode,
    data: VecDeque<T>
}

impl<T> InnerQueue<T> {
    fn new(mode: QueueMode) -> InnerQueue<T> {
        InnerQueue {
            mode,
            data: VecDeque::new()
        }
    }
    pub fn push(&mut self, item : T) {
        self.data.push_front(item)
    }
    
    pub fn pop(&mut self) -> Option<T> {
         match self.mode {
            QueueMode::FIFO => {
                self.data.pop_back()
            }
            QueueMode::LIFO => {
                self.data.pop_front()
            }
        }
    }
    
    pub fn peek(&self) -> Option<&T> {
        match self.mode {
            QueueMode::FIFO => {
                self.data.back()
            }
            QueueMode::LIFO => {
                self.data.front()
            }
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
        let queue : Queue<usize> = Queue::new(QueueMode::FIFO);
        queue.push(1);
        queue.push(2);
        queue.push(3);
        assert_eq!(1, queue.pop().unwrap());
        assert_eq!(2, queue.pop().unwrap());
        assert_eq!(3, queue.pop().unwrap());
    }

    #[test]
    fn test_lifo() {
        let queue : Queue<usize> = Queue::new(QueueMode::LIFO);
        queue.push(1);
        queue.push(2);
        queue.push(3);
        assert_eq!(3, queue.pop().unwrap());
        assert_eq!(2, queue.pop().unwrap());
        assert_eq!(1, queue.pop().unwrap());
    }
    #[test]
    fn test_peek_and_len() {
        let lifo : Queue<usize> = Queue::new(QueueMode::LIFO);
        lifo.push(10);
        
        let value = lifo.peek(|v| *v).unwrap();
        assert_eq!(value, 10);

        let fifo : Queue<usize> = Queue::new(QueueMode::LIFO);
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
        let handle = thread::spawn( move || {
            queue_clone.pop_blocking()
        });

        thread::sleep(Duration::from_millis(100));
        queue.push(5);

        let value = handle.join().unwrap();
        assert_eq!(value, 5);
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
}

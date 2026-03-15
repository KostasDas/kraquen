# Kraquen

A thread-safe, ergonomic and generic queue for Rust. 

Kraquen is designed for concurrent applications where you need reliable communication between producers and consumers. It operates in both **FIFO** (First-In, First-Out) and **LIFO** (Last-In, First-Out) modes. Backed by a `Mutex` and `Condvar`, it provides blocking waits, and features an optional **Lossy Ring Buffer** mode to prevent memory exhaustion without deadlocking producers.

## Key Features

* **Thread-Safe by Default:** Already wrapped in `Arc` `Mutex`
* **Dual Modes:** Support for both standard FIFO pipelines and LIFO stack patterns.
* **Bounded Eviction (Ring Buffer):** Cap the maximum size of your queue. If full, new pushes seamlessly evict old data instead of deadlocking your producer threads.
* **Ergonomic Retrievals:** Choose between non-blocking (`pop`), blocking (`pop_blocking`), and timed blocking (`pop_timeout`) retrievals.
* **Graceful Shutdown:** Safely close the queue to prevent new pushes while allowing consumers to drain the remaining items.
* **Zero-Copy Peeking:** Inspect the next item in line using a closure without removing it.

---

## Quick Start

Add Kraquen to your `Cargo.toml`:

```toml
[dependencies]
kraquen = "0.1.0"
```

### Basic Usage (Unbounded)

```rust
use kraquen::{Queue, QueueMode};

// Create a new unbounded FIFO queue
let fifo = Queue::new(QueueMode::FIFO);
fifo.push("Task 1"); // Returns None (nothing was evicted)
fifo.push("Task 2");

assert_eq!(fifo.pop(), Some("Task 1"));
```

### Bounded Capacity (Eviction / Ring Buffer)

If you have a fast producer and a slow consumer, you can limit the queue size to prevent Out-Of-Memory (OOM) crashes. Kraquen prioritizes liveness: producers *never* block. If the queue is full, the next scheduled item is evicted to make room and returned to you.

```rust
use kraquen::{Queue, QueueMode};

// Create a FIFO queue that holds a maximum of 2 items
let queue = Queue::with_capacity(QueueMode::FIFO, 2);

queue.push("A");
queue.push("B");

// The queue is full! Pushing "C" will evict the oldest item ("A").
let evicted = queue.push("C");
assert_eq!(evicted, Some("A"));

// The queue now contains ["B", "C"]
assert_eq!(queue.pop(), Some("B"));
```

### Multithreaded Worker Pool (Blocking)

Kraquen shines when used to coordinate work between threads. 

```rust
use kraquen::{Queue, QueueMode};
use std::thread;

let queue = Queue::new(QueueMode::FIFO);
let worker_queue = queue.clone();

// Spawn a worker thread
let worker = thread::spawn(move || {
    // Blocks efficiently using condvar until a task arrives or queue shuts down
    while let Some(task) = worker_queue.pop_blocking() {
        println!("Processing: {}", task);
    }
    println!("Queue shut down, worker exiting.");
});

// Push work from the main thread
queue.push("Generate Report");
queue.push("Send Email");

// Shut down to tell the worker we are done
queue.shutdown();
worker.join().unwrap();
```

### Handling Shutdowns and Timeouts

Use `try_push` to handle shutdown states safely, and `pop_timeout` to prevent threads from waiting indefinitely.

```rust
use kraquen::{Queue, QueueMode};
use std::time::Duration;

let queue: Queue<i32> = Queue::new(QueueMode::FIFO);

// 1. Timeouts
match queue.pop_timeout(Duration::from_secs(60)) {
    Some(item) => println!("Got an item: {}", item),
    None => println!("Timed out waiting for data!"),
}

// 2. Safe Pushing during Shutdown
queue.shutdown();

// push() will panic on a shut down queue. 
// try_push() safely returns the item in an Err so you don't lose the data.
match queue.try_push(1) {
    Ok(_) => println!("Pushed successfully"),
    Err(returned_item) => println!("Queue closed. Item {} was returned.", returned_item),
}
```

---

##  API Reference

| Method | Description |
| :--- | :--- |
| `new(mode)` | Creates a new unbounded queue with the specified `QueueMode`. |
| `with_capacity(mode, c)` | Creates a new bounded queue with a maximum capacity `c`. |
| `push(item)` | Adds an item. Returns `Option<T>` containing an evicted item if full. **Panics** if shut down. |
| `try_push(item)` | Safely attempts to add an item. Returns `Ok(Option<T>)` with evicted item, or `Err(item)` if shut down. |
| `pop()` | Retrieves an item immediately, or returns `None` if empty. |
| `pop_blocking()` | Waits for an item. Returns `None` only if shut down and empty. |
| `pop_timeout(d)` | Waits up to `d`. Returns `None` if timed out or shut down. |
| `peek(closure)` | Applies a closure to the next item in the queue without removing it. |
| `len()` | Returns the number of items currently in the queue. |
| `is_empty()` | Returns `true` if the queue has zero items. |
| `clear()` | Removes all items from the queue. |
| `shutdown()` | Closes the queue to new pushes and wakes all waiting threads. |

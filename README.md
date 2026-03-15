# Kraquen 🦑

A fast, thread-safe, and highly ergonomic generic queue for Rust. 

Kraquen is designed for concurrent applications where you need reliable communication between producers and consumers. It operates seamlessly in both **FIFO** (First-In, First-Out) and **LIFO** (Last-In, First-Out) modes, backed by a `Mutex` and `Condvar` for highly efficient blocking waits.

## ✨ Key Features

* **Thread-Safe by Default:** Easily share the queue across multiple threads without wrapping it in additional `Arc`s or `Mutex`es.
* **Dual Modes:** Support for both standard FIFO pipelines and LIFO stack patterns.
* **Ergonomic Retrievals:**
    * `pop()`: Non-blocking retrieval (returns immediately).
    * `pop_blocking()`: Efficiently puts the thread to sleep until data arrives.
    * `pop_timeout()`: Blocks until data arrives or a specific duration elapses.
* **Graceful Shutdown:** Safely close the queue to prevent new pushes while allowing consumers to drain the remaining items and exit cleanly.
* **Zero-Copy Peeking:** Inspect the next item in line using a closure without removing it from the queue.

---

## 🚀 Quick Start

Add Kraquen to your `Cargo.toml`:

```toml
[dependencies]
kraquen = "0.1.0"```

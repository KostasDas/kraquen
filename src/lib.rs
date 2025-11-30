use std::collections::VecDeque;

pub enum QueueMode {
    FIFO,
    LIFO
}
pub struct Queue<T> {
    mode: QueueMode,
    data: VecDeque<T>
}

impl<T> Queue<T> {
    pub fn new(mode: QueueMode) -> Queue<T> {
        Queue {
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
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_push() {
        let mut queue: Queue<usize> = Queue::new(QueueMode::FIFO);
        queue.push(1);
        let element = queue.pop();
        assert_eq!(1, element.unwrap());
        assert_eq!(None, queue.pop());
    }
    #[test]
    fn test_fifo() {
        let mut queue : Queue<usize> = Queue::new(QueueMode::FIFO);
        queue.push(1);
        queue.push(2);
        queue.push(3);
        assert_eq!(1, queue.pop().unwrap());
        assert_eq!(2, queue.pop().unwrap());
        assert_eq!(3, queue.pop().unwrap());
    }

    #[test]
    fn test_lifo() {
        let mut queue : Queue<usize> = Queue::new(QueueMode::LIFO);
        queue.push(1);
        queue.push(2);
        queue.push(3);
        assert_eq!(3, queue.pop().unwrap());
        assert_eq!(2, queue.pop().unwrap());
        assert_eq!(1, queue.pop().unwrap());
    }
}

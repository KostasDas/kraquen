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
    
    pub fn len(&self) -> usize {
        self.data.len()
    }
    
    pub fn is_empty(&self) -> bool {
        self.data.is_empty()
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
    
    pub fn clear(&mut self) {
        self.data.clear()
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
    #[test]
    fn test_peek_and_len() {
        let mut lifo : Queue<usize> = Queue::new(QueueMode::LIFO);
        lifo.push(1);
        lifo.push(2);
        assert_eq!(2, *lifo.peek().unwrap());
        assert_eq!(2, lifo.len());

        let mut fifo : Queue<usize> = Queue::new(QueueMode::FIFO);
        fifo.push(1);
        fifo.push(2);
        assert_eq!(1, *fifo.peek().unwrap());
        assert_eq!(2, fifo.len());
    }
    
    #[test]
    fn test_clear_and_len() {
        let mut lifo : Queue<usize> = Queue::new(QueueMode::LIFO);
        lifo.push(1);
        lifo.push(2);
        lifo.clear();
        assert_eq!(0, lifo.len());

        let mut fifo : Queue<usize> = Queue::new(QueueMode::FIFO);
        fifo.push(1);
        fifo.push(2);
        fifo.clear();
        assert_eq!(0, fifo.len());
    }
}

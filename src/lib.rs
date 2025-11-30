use std::collections::VecDeque;

pub enum QueueMode {
    FIFO,
    LIFO
}
pub struct Queue<T> {
    mode: QueueMode,
    data: VecDeque<T>
}

impl<T: 'static> IntoIterator for Queue<T> {
    type Item = T;
    type IntoIter = Box<dyn Iterator<Item = T>>;

    fn into_iter(self) -> Self::IntoIter {
        match self.mode {
            QueueMode::FIFO => {
                Box::new(self.data.into_iter().rev())
            }
            QueueMode::LIFO => {
                Box::new(self.data.into_iter())
            }
        }
    }
}
impl<'a, T> IntoIterator for &'a Queue<T> {
    type Item = &'a T;
    type IntoIter = Box<dyn Iterator<Item = &'a T> + 'a>;

    fn into_iter(self) -> Self::IntoIter {
        self.iter()
    }
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
    
    pub fn iter(&self) -> Box<dyn Iterator<Item = &T> + '_> {
        match self.mode {
            QueueMode::FIFO => {
                Box::new(self.data.iter().rev())
            }
            QueueMode::LIFO => {
                Box::new(self.data.iter())
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

    #[test]
    fn test_iter_borrow_fifo() {
        let mut queue = Queue::new(QueueMode::FIFO);
        queue.push(10);
        queue.push(20);
        queue.push(30);

        let collected: Vec<&i32> = queue.iter().collect();
        assert_eq!(collected, vec![&10, &20, &30]);

        // Ensure queue is still alive (iter borrowed it)
        assert_eq!(queue.len(), 3);
    }

    #[test]
    fn test_iter_borrow_lifo() {
        let mut queue = Queue::new(QueueMode::LIFO);
        queue.push(10);
        queue.push(20);
        queue.push(30);

        // LIFO Expectation: 30, 20, 10
        let collected: Vec<&i32> = queue.iter().collect();
        assert_eq!(collected, vec![&30, &20, &10]);
    }

    #[test]
    fn test_into_iter_fifo() {
        let mut queue = Queue::new(QueueMode::FIFO);
        queue.push("a");
        queue.push("b");

        // Use the trait implicitly in a loop
        let mut result = Vec::new();
        for item in queue {
            result.push(item);
        }
        assert_eq!(result, vec!["a", "b"]);
    }

    #[test]
    fn test_into_iter_lifo() {
        let mut queue = Queue::new(QueueMode::LIFO);
        queue.push("a");
        queue.push("b");

        let result: Vec<&str> = queue.into_iter().collect();

        assert_eq!(result, vec!["b", "a"]);
    }

    #[test]
    fn test_into_iter_reference_fifo() {
        let mut queue = Queue::new(QueueMode::FIFO);
        queue.push("a");
        queue.push("b");
        
        let mut result = Vec::new();
        for item in &queue {
            result.push(item);
        }
        assert_eq!(result, vec![&"a", &"b"]);
        // queue is not destroyed in this case
        assert_eq!(2, queue.len())
    }

    #[test]
    fn test_into_iter_reference_lifo() {
        let mut queue = Queue::new(QueueMode::LIFO);
        queue.push("a");
        queue.push("b");

        let mut result = Vec::new();
        for item in &queue {
            result.push(item);
        }

        assert_eq!(result, vec![&"b", &"a"]);
        assert_eq!(2, queue.len())
    }
}

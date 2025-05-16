pub struct Stack<T> {
    backend: Vec<T>,
}

impl<T> Stack<T> {
    #[must_use]
    pub(crate) fn new() -> Self {
        Stack { backend: Vec::new() }
    }

    pub(crate) fn push(&mut self, item: T) {
        self.backend.push(item);
    }

    pub fn pop(&mut self) -> T {
        assert_eq!(self.backend.is_empty(), false, "can't pop from empty stack");
        self.backend.pop().unwrap()
    }

    #[must_use]
    pub fn size(&self) -> usize {
        self.backend.len()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_push_pop() {
        let mut system_under_test: Stack<i32> = Stack::new();
        
        system_under_test.push(1);
        system_under_test.push(2);
        
        assert_eq!(system_under_test.pop(), 2);
        assert_eq!(system_under_test.pop(), 1);
    }

    #[test]
    fn test_size() {
        let mut system_under_test: Stack<String> = Stack::new();
        assert_eq!(system_under_test.size(), 0);
        
        system_under_test.push("hello".to_string());
        assert_eq!(system_under_test.size(), 1);
        
        system_under_test.push("world".to_string());
        assert_eq!(system_under_test.size(), 2);
        
        system_under_test.pop();
        assert_eq!(system_under_test.size(), 1);
        
        system_under_test.pop();
        assert_eq!(system_under_test.size(), 0);
    }

    #[test]
    #[should_panic]
    fn test_pop_empty() {
        let mut system_under_test: Stack<f64> = Stack::new();
        system_under_test.pop();
    }
}

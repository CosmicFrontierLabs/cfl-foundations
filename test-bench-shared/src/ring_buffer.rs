//! A fixed-capacity ring buffer that drops oldest elements when full.

use std::collections::VecDeque;

/// A fixed-capacity ring buffer that automatically drops the oldest element
/// when pushing to a full buffer.
///
/// This is useful for maintaining rolling windows of data, such as
/// measurement histories, log buffers, or time series data.
#[derive(Debug, Clone, PartialEq)]
pub struct RingBuffer<T> {
    data: VecDeque<T>,
    capacity: usize,
}

impl<T> RingBuffer<T> {
    /// Create a new ring buffer with the specified capacity.
    ///
    /// # Panics
    /// Panics if capacity is 0.
    pub fn new(capacity: usize) -> Self {
        assert!(capacity > 0, "RingBuffer capacity must be > 0");
        Self {
            data: VecDeque::with_capacity(capacity),
            capacity,
        }
    }

    /// Push a value onto the buffer, dropping the oldest if at capacity.
    pub fn push(&mut self, value: T) {
        if self.data.len() >= self.capacity {
            self.data.pop_front();
        }
        self.data.push_back(value);
    }

    /// Returns the number of elements in the buffer.
    pub fn len(&self) -> usize {
        self.data.len()
    }

    /// Returns true if the buffer is empty.
    pub fn is_empty(&self) -> bool {
        self.data.is_empty()
    }

    /// Returns the maximum capacity of the buffer.
    pub fn capacity(&self) -> usize {
        self.capacity
    }

    /// Returns an iterator over the elements in order (oldest to newest).
    pub fn iter(&self) -> impl Iterator<Item = &T> {
        self.data.iter()
    }

    /// Clear all elements from the buffer.
    pub fn clear(&mut self) {
        self.data.clear();
    }

    /// Returns a reference to the oldest element, if any.
    pub fn front(&self) -> Option<&T> {
        self.data.front()
    }

    /// Returns a reference to the newest element, if any.
    pub fn back(&self) -> Option<&T> {
        self.data.back()
    }

    /// Returns a reference to the underlying VecDeque.
    ///
    /// This is useful for compatibility with code that expects a VecDeque.
    pub fn as_deque(&self) -> &VecDeque<T> {
        &self.data
    }
}

impl<T> Default for RingBuffer<T> {
    fn default() -> Self {
        Self::new(64)
    }
}

impl<'a, T> IntoIterator for &'a RingBuffer<T> {
    type Item = &'a T;
    type IntoIter = std::collections::vec_deque::Iter<'a, T>;

    fn into_iter(self) -> Self::IntoIter {
        self.data.iter()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_basic_push_and_len() {
        let mut buf = RingBuffer::new(3);
        assert_eq!(buf.len(), 0);
        assert!(buf.is_empty());

        buf.push(1);
        assert_eq!(buf.len(), 1);
        assert!(!buf.is_empty());

        buf.push(2);
        buf.push(3);
        assert_eq!(buf.len(), 3);
    }

    #[test]
    fn test_overflow_drops_oldest() {
        let mut buf = RingBuffer::new(3);
        buf.push(1);
        buf.push(2);
        buf.push(3);

        // Push 4th element, should drop 1
        buf.push(4);
        assert_eq!(buf.len(), 3);

        let items: Vec<_> = buf.iter().copied().collect();
        assert_eq!(items, vec![2, 3, 4]);
    }

    #[test]
    fn test_front_and_back() {
        let mut buf = RingBuffer::new(3);
        assert!(buf.front().is_none());
        assert!(buf.back().is_none());

        buf.push(1);
        assert_eq!(buf.front(), Some(&1));
        assert_eq!(buf.back(), Some(&1));

        buf.push(2);
        buf.push(3);
        assert_eq!(buf.front(), Some(&1));
        assert_eq!(buf.back(), Some(&3));

        buf.push(4);
        assert_eq!(buf.front(), Some(&2));
        assert_eq!(buf.back(), Some(&4));
    }

    #[test]
    fn test_clear() {
        let mut buf = RingBuffer::new(3);
        buf.push(1);
        buf.push(2);
        buf.clear();
        assert!(buf.is_empty());
        assert_eq!(buf.len(), 0);
    }

    #[test]
    fn test_capacity() {
        let buf: RingBuffer<i32> = RingBuffer::new(10);
        assert_eq!(buf.capacity(), 10);
    }

    #[test]
    fn test_into_iterator() {
        let mut buf = RingBuffer::new(3);
        buf.push(1);
        buf.push(2);
        buf.push(3);

        let sum: i32 = (&buf).into_iter().sum();
        assert_eq!(sum, 6);
    }

    #[test]
    #[should_panic(expected = "capacity must be > 0")]
    fn test_zero_capacity_panics() {
        let _buf: RingBuffer<i32> = RingBuffer::new(0);
    }
}

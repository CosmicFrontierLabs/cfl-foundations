//! Frame ring buffer for pre-allocated camera frame storage
//!
//! Provides a circular buffer of pre-allocated Array2<u16> frames that can be
//! reused without allocating on each camera capture. Supports dynamic reallocation
//! when ROI dimensions change.
//!
//! Tracks separate read and write positions to detect when writes are outpacing reads.

use ndarray::Array2;
use std::error::Error;
use std::fmt;
use std::sync::{Arc, Condvar, Mutex};
use std::time::Duration;

/// Error type for ring buffer write operations
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum BufferWriteError {
    /// Buffer is full - write would overwrite unread data
    BufferFull { unread_frames: usize },
}

impl fmt::Display for BufferWriteError {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self {
            BufferWriteError::BufferFull { unread_frames } => {
                write!(
                    f,
                    "Ring buffer full: would overwrite {unread_frames} unread frames"
                )
            }
        }
    }
}

impl Error for BufferWriteError {}

/// Error type for ring buffer read operations
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum BufferReadError {
    /// No frames available to read
    NoFramesAvailable,
    /// Timeout waiting for readable frame
    Timeout,
}

impl fmt::Display for BufferReadError {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self {
            BufferReadError::NoFramesAvailable => {
                write!(f, "No frames available to read")
            }
            BufferReadError::Timeout => {
                write!(f, "Timeout waiting for readable frame")
            }
        }
    }
}

impl Error for BufferReadError {}

/// Result type for ring buffer write operations
pub type BufferWriteResult<T> = Result<T, BufferWriteError>;

/// Result type for ring buffer read operations
pub type BufferReadResult<T> = Result<T, BufferReadError>;

/// Ring buffer for pre-allocated camera frames
///
/// This structure maintains a fixed number of pre-allocated frame buffers that
/// can be reused in a circular fashion. When ROI dimensions change, all buffers
/// are reallocated to the new size.
#[derive(Debug)]
pub struct FrameRingBuffer {
    /// Pre-allocated frame buffers
    buffers: Vec<Array2<u16>>,
    /// Number of buffers in the ring
    capacity: usize,
    /// Next position to write to
    write_idx: usize,
    /// Next position to read from
    read_idx: usize,
    /// Current buffer height (rows)
    rows: usize,
    /// Current buffer width (columns)
    cols: usize,
    /// Condition variable for notifying readers when frames are available
    condvar: Arc<Condvar>,
    /// Mutex for condvar  (stores a dummy bool, just for Condvar wait)
    condvar_mutex: Arc<Mutex<()>>,
}

impl FrameRingBuffer {
    /// Create a new ring buffer with pre-allocated frames
    ///
    /// # Arguments
    /// * `capacity` - Number of buffers to allocate
    /// * `rows` - Initial height of each buffer (pixels)
    /// * `cols` - Initial width of each buffer (pixels)
    ///
    /// # Returns
    /// A new FrameRingBuffer with all buffers pre-allocated and zero-initialized
    ///
    /// # Panics
    /// Panics if capacity is 0
    pub fn new(capacity: usize, rows: usize, cols: usize) -> Self {
        assert!(capacity > 0, "Ring buffer capacity must be greater than 0");
        assert!(
            capacity >= 2,
            "Ring buffer capacity must be at least 2 for read/write tracking"
        );

        let buffers = (0..capacity)
            .map(|_| Array2::<u16>::zeros((rows, cols)))
            .collect();

        Self {
            buffers,
            capacity,
            write_idx: 0,
            read_idx: 0,
            rows,
            cols,
            condvar: Arc::new(Condvar::new()),
            condvar_mutex: Arc::new(Mutex::new(())),
        }
    }

    /// Get the next buffer for writing (mutable access)
    ///
    /// Returns a mutable reference to the next buffer in the ring.
    /// Checks if write would overwrite unread data and returns error if so.
    /// Notifies waiting readers via condvar after successful write.
    ///
    /// # Returns
    /// * `Ok(&mut Array2<u16>)` - Mutable reference to next write buffer
    /// * `Err(BufferWriteError::BufferFull)` - Would overwrite unread frames
    pub fn next_buffer_mut(&mut self) -> BufferWriteResult<&mut Array2<u16>> {
        let next_write = (self.write_idx + 1) % self.capacity;

        if next_write == self.read_idx {
            let unread = self.unread_count();
            return Err(BufferWriteError::BufferFull {
                unread_frames: unread,
            });
        }

        let buf = &mut self.buffers[self.write_idx];
        self.write_idx = next_write;

        // Notify waiting readers that a frame is available
        self.condvar.notify_one();

        Ok(buf)
    }

    /// Count number of unread frames in the buffer
    ///
    /// # Returns
    /// Number of frames written but not yet read
    pub fn unread_count(&self) -> usize {
        if self.write_idx >= self.read_idx {
            self.write_idx - self.read_idx
        } else {
            self.capacity - self.read_idx + self.write_idx
        }
    }

    /// Check if buffer is full (write would overwrite unread data)
    ///
    /// # Returns
    /// true if next write would overwrite unread frames
    pub fn is_full(&self) -> bool {
        let next_write = (self.write_idx + 1) % self.capacity;
        next_write == self.read_idx
    }

    /// Check if buffer is empty (no unread frames)
    ///
    /// # Returns
    /// true if no frames available to read
    pub fn is_empty(&self) -> bool {
        self.write_idx == self.read_idx
    }

    /// Read the next frame from the buffer
    ///
    /// Advances the read position and returns a reference to the frame.
    ///
    /// # Returns
    /// * `Ok(&Array2<u16>)` - Reference to next unread frame
    /// * `Err(BufferReadError::NoFramesAvailable)` - No unread frames
    pub fn read_next(&mut self) -> BufferReadResult<&Array2<u16>> {
        if self.is_empty() {
            return Err(BufferReadError::NoFramesAvailable);
        }

        let buf = &self.buffers[self.read_idx];
        self.read_idx = (self.read_idx + 1) % self.capacity;
        Ok(buf)
    }

    /// Get the last written buffer (immutable access)
    ///
    /// Returns a reference to the most recently written buffer without
    /// advancing positions. Returns None if no frames have been written.
    ///
    /// # Returns
    /// Reference to the last written buffer, or None if empty
    pub fn last_written(&self) -> Option<&Array2<u16>> {
        if self.is_empty() {
            return None;
        }

        let idx = if self.write_idx == 0 {
            self.capacity - 1
        } else {
            self.write_idx - 1
        };
        Some(&self.buffers[idx])
    }

    /// Get the last read buffer (immutable access)
    ///
    /// Returns a reference to the most recently read buffer without
    /// advancing positions. Returns None if no frames have been read.
    ///
    /// # Returns
    /// Reference to the last read buffer, or None if nothing read yet
    pub fn last_read(&self) -> Option<&Array2<u16>> {
        // If read_idx hasn't moved from initial position and write has, nothing read yet
        if self.read_idx == 0 && self.write_idx > 0 {
            return None;
        }

        // If buffer is completely empty, nothing to read
        if self.is_empty() {
            return None;
        }

        // Return buffer before current read position
        let idx = if self.read_idx == 0 {
            self.capacity - 1
        } else {
            self.read_idx - 1
        };
        Some(&self.buffers[idx])
    }

    /// Get a specific buffer by index
    ///
    /// # Arguments
    /// * `idx` - Buffer index (0 to capacity-1)
    ///
    /// # Returns
    /// Reference to the buffer at the given index
    ///
    /// # Panics
    /// Panics if idx >= capacity
    pub fn get(&self, idx: usize) -> &Array2<u16> {
        &self.buffers[idx]
    }

    /// Get a specific buffer by index (mutable access)
    ///
    /// # Arguments
    /// * `idx` - Buffer index (0 to capacity-1)
    ///
    /// # Returns
    /// Mutable reference to the buffer at the given index
    ///
    /// # Panics
    /// Panics if idx >= capacity
    pub fn get_mut(&mut self, idx: usize) -> &mut Array2<u16> {
        &mut self.buffers[idx]
    }

    /// Resize all buffers to new dimensions
    ///
    /// Reallocates all buffers when ROI dimensions change. This is an expensive
    /// operation and should only be called when necessary. Resets read/write positions.
    ///
    /// # Arguments
    /// * `rows` - New height (pixels)
    /// * `cols` - New width (pixels)
    pub fn resize(&mut self, rows: usize, cols: usize) {
        if self.rows == rows && self.cols == cols {
            return;
        }

        self.buffers = (0..self.capacity)
            .map(|_| Array2::<u16>::zeros((rows, cols)))
            .collect();

        self.rows = rows;
        self.cols = cols;
        self.write_idx = 0;
        self.read_idx = 0;
    }

    /// Get the number of buffers in the ring
    ///
    /// # Returns
    /// Ring buffer capacity
    pub fn capacity(&self) -> usize {
        self.capacity
    }

    /// Get current buffer dimensions
    ///
    /// # Returns
    /// Tuple of (rows, cols)
    pub fn dimensions(&self) -> (usize, usize) {
        (self.rows, self.cols)
    }

    /// Get current height in pixels
    ///
    /// # Returns
    /// Buffer height (rows)
    pub fn rows(&self) -> usize {
        self.rows
    }

    /// Get current width in pixels
    ///
    /// # Returns
    /// Buffer width (columns)
    pub fn cols(&self) -> usize {
        self.cols
    }

    /// Reset the ring buffer positions
    ///
    /// This does not clear buffer contents, only resets read and write indices.
    /// After reset, all frames are considered "unread" again.
    pub fn reset(&mut self) {
        self.write_idx = 0;
        self.read_idx = 0;
    }

    /// Wait for a readable frame with timeout
    ///
    /// Blocks until a frame is available or timeout expires.
    /// Uses condition variable to efficiently wait for writers to produce frames.
    ///
    /// # Arguments
    /// * `timeout` - Maximum duration to wait for a frame
    ///
    /// # Returns
    /// * `Ok(())` - Frame is now available, call read_next() to retrieve it
    /// * `Err(BufferReadError::Timeout)` - Timeout expired with no frame available
    pub fn wait_for_readable(&self, timeout: Duration) -> BufferReadResult<()> {
        let guard = self.condvar_mutex.lock().unwrap();

        // Wait for notification with timeout
        let (_guard, timeout_result) = self.condvar.wait_timeout(guard, timeout).unwrap();

        // Check if frames are available after wait
        if timeout_result.timed_out() && self.is_empty() {
            return Err(BufferReadError::Timeout);
        }

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_ring_buffer_creation() {
        let ring = FrameRingBuffer::new(3, 100, 200);
        assert_eq!(ring.capacity(), 3);
        assert_eq!(ring.dimensions(), (100, 200));
        assert_eq!(ring.rows(), 100);
        assert_eq!(ring.cols(), 200);
        assert!(ring.is_empty());
        assert_eq!(ring.unread_count(), 0);
    }

    #[test]
    #[should_panic(expected = "Ring buffer capacity must be greater than 0")]
    fn test_zero_capacity_panics() {
        let _ring = FrameRingBuffer::new(0, 100, 100);
    }

    #[test]
    #[should_panic(expected = "Ring buffer capacity must be at least 2")]
    fn test_single_capacity_panics() {
        let _ring = FrameRingBuffer::new(1, 100, 100);
    }

    #[test]
    fn test_write_and_read() {
        let mut ring = FrameRingBuffer::new(3, 10, 10);

        // Write first frame
        let buf1 = ring.next_buffer_mut().unwrap();
        buf1[[0, 0]] = 1000;
        assert_eq!(ring.unread_count(), 1);
        assert!(!ring.is_empty());

        // Write second frame
        let buf2 = ring.next_buffer_mut().unwrap();
        buf2[[0, 0]] = 2000;
        assert_eq!(ring.unread_count(), 2);

        // Read first frame
        let read1 = ring.read_next().unwrap();
        assert_eq!(read1[[0, 0]], 1000);
        assert_eq!(ring.unread_count(), 1);

        // Read second frame
        let read2 = ring.read_next().unwrap();
        assert_eq!(read2[[0, 0]], 2000);
        assert_eq!(ring.unread_count(), 0);
        assert!(ring.is_empty());
    }

    #[test]
    fn test_buffer_full_error() {
        let mut ring = FrameRingBuffer::new(3, 10, 10);

        // Fill buffer (capacity-1 frames)
        ring.next_buffer_mut().unwrap()[[0, 0]] = 1;
        ring.next_buffer_mut().unwrap()[[0, 0]] = 2;

        // Buffer is now full (can't write without overwriting)
        assert!(ring.is_full());
        let err = ring.next_buffer_mut().unwrap_err();
        match err {
            BufferWriteError::BufferFull { unread_frames } => {
                assert_eq!(unread_frames, 2);
            }
        }
    }

    #[test]
    fn test_no_frames_available_error() {
        let mut ring = FrameRingBuffer::new(3, 10, 10);

        // Try to read from empty buffer
        let err = ring.read_next().unwrap_err();
        assert!(matches!(err, BufferReadError::NoFramesAvailable));
    }

    #[test]
    fn test_last_written_and_read() {
        let mut ring = FrameRingBuffer::new(3, 10, 10);

        // Initially nothing written
        assert!(ring.last_written().is_none());
        assert!(ring.last_read().is_none());

        // Write a frame
        ring.next_buffer_mut().unwrap()[[0, 0]] = 100;
        assert_eq!(ring.last_written().unwrap()[[0, 0]], 100);
        assert!(ring.last_read().is_none());

        // Write another
        ring.next_buffer_mut().unwrap()[[0, 0]] = 200;
        assert_eq!(ring.last_written().unwrap()[[0, 0]], 200);

        // Read one
        ring.read_next().unwrap();
        assert_eq!(ring.last_read().unwrap()[[0, 0]], 100);
        assert_eq!(ring.last_written().unwrap()[[0, 0]], 200);
    }

    #[test]
    fn test_wrapping_behavior() {
        let mut ring = FrameRingBuffer::new(3, 10, 10);

        // Write 2, read 2, write 2 more (tests wrapping)
        ring.next_buffer_mut().unwrap()[[0, 0]] = 1;
        ring.next_buffer_mut().unwrap()[[0, 0]] = 2;

        assert_eq!(ring.read_next().unwrap()[[0, 0]], 1);
        assert_eq!(ring.read_next().unwrap()[[0, 0]], 2);

        // Now can write again (buffer has space)
        assert!(!ring.is_full());
        ring.next_buffer_mut().unwrap()[[0, 0]] = 3;
        ring.next_buffer_mut().unwrap()[[0, 0]] = 4;

        // Verify wrapped writes
        assert_eq!(ring.read_next().unwrap()[[0, 0]], 3);
        assert_eq!(ring.read_next().unwrap()[[0, 0]], 4);
    }

    #[test]
    fn test_unread_count_wrapping() {
        let mut ring = FrameRingBuffer::new(4, 10, 10);

        // Write 2
        ring.next_buffer_mut().unwrap();
        ring.next_buffer_mut().unwrap();
        assert_eq!(ring.unread_count(), 2);

        // Read 1
        ring.read_next().unwrap();
        assert_eq!(ring.unread_count(), 1);

        // Write 2 more (tests wrap around)
        ring.next_buffer_mut().unwrap();
        ring.next_buffer_mut().unwrap();
        assert_eq!(ring.unread_count(), 3);
    }

    #[test]
    fn test_resize() {
        let mut ring = FrameRingBuffer::new(2, 10, 10);

        let buf = ring.next_buffer_mut().unwrap();
        buf[[0, 0]] = 999;

        // Resize to larger dimensions
        ring.resize(20, 30);
        assert_eq!(ring.dimensions(), (20, 30));
        assert!(ring.is_empty());

        // Old data should be gone (buffers reallocated)
        let buf = ring.next_buffer_mut().unwrap();
        assert_eq!(buf[[0, 0]], 0);
        assert_eq!(buf.shape(), &[20, 30]);
    }

    #[test]
    fn test_resize_same_dimensions() {
        let mut ring = FrameRingBuffer::new(2, 10, 10);

        let buf = ring.next_buffer_mut().unwrap();
        buf[[0, 0]] = 999;

        // Resize to same dimensions should be no-op
        ring.resize(10, 10);
        assert_eq!(ring.dimensions(), (10, 10));

        // Data should still be there since resize was no-op
        assert_eq!(ring.get(0)[[0, 0]], 999);
    }

    #[test]
    fn test_reset() {
        let mut ring = FrameRingBuffer::new(3, 10, 10);

        ring.next_buffer_mut().unwrap();
        ring.next_buffer_mut().unwrap();
        assert_eq!(ring.unread_count(), 2);

        ring.reset();
        assert!(ring.is_empty());
        assert_eq!(ring.unread_count(), 0);

        // Should be able to write from beginning
        let buf = ring.next_buffer_mut().unwrap();
        buf[[0, 0]] = 111;
        assert_eq!(ring.get(0)[[0, 0]], 111);
    }

    #[test]
    fn test_direct_buffer_access() {
        let mut ring = FrameRingBuffer::new(3, 10, 10);

        // Use direct access to set values
        ring.get_mut(0)[[0, 0]] = 100;
        ring.get_mut(1)[[0, 0]] = 200;
        ring.get_mut(2)[[0, 0]] = 300;

        // Verify via get
        assert_eq!(ring.get(0)[[0, 0]], 100);
        assert_eq!(ring.get(1)[[0, 0]], 200);
        assert_eq!(ring.get(2)[[0, 0]], 300);
    }
}

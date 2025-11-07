//! Image dimensions and size utilities

use ndarray::Array2;
use serde::{Deserialize, Serialize};
use std::fmt;

/// Image dimensions structure
///
/// Represents the width and height of an image sensor or frame.
/// Provides convenience methods for creating arrays and calculations.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct ImageSize {
    /// Image width in pixels
    pub width: usize,
    /// Image height in pixels
    pub height: usize,
}

impl ImageSize {
    /// Create a new ImageSize
    pub fn from_width_height(width: usize, height: usize) -> Self {
        Self { width, height }
    }

    /// Create an empty array with this size
    ///
    /// Returns an ndarray Array2 of zeros with shape (height, width).
    /// Note the row-major ordering convention: rows (height) come first.
    pub fn empty_array<T>(&self) -> Array2<T>
    where
        T: ndarray::NdFloat + Default,
    {
        Array2::default((self.height, self.width))
    }

    /// Create an empty u16 array with this size
    ///
    /// Specialized version for u16 which is the most common camera data type.
    pub fn empty_array_u16(&self) -> Array2<u16> {
        Array2::zeros((self.height, self.width))
    }

    /// Get total number of pixels
    pub fn pixel_count(&self) -> usize {
        self.width * self.height
    }

    /// Convert to tuple (width, height)
    pub fn to_tuple(&self) -> (usize, usize) {
        (self.width, self.height)
    }

    /// Create from tuple (width, height)
    pub fn from_tuple(dimensions: (usize, usize)) -> Self {
        Self {
            width: dimensions.0,
            height: dimensions.1,
        }
    }
}

impl From<(usize, usize)> for ImageSize {
    fn from(dimensions: (usize, usize)) -> Self {
        Self::from_tuple(dimensions)
    }
}

impl From<ImageSize> for (usize, usize) {
    fn from(size: ImageSize) -> Self {
        size.to_tuple()
    }
}

impl fmt::Display for ImageSize {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "{}x{}", self.width, self.height)
    }
}

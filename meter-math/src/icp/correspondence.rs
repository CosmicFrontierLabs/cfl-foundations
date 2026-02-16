//! Point correspondence via nearest-neighbor matching.
//!
//! This module implements the correspondence step of ICP by finding
//! the nearest target point for each source point.

use nalgebra::Vector2;

/// Finds the closest target point for each source point using brute-force search.
///
/// Returns a vector of (source_index, target_index) pairs representing closest matches.
///
/// Time complexity: O(n × m) where n = source points, m = target points.
// TODO: accelerate with a BSP tree or KD-tree for large point sets
pub(super) fn find_closest_points(
    source_points: &[Vector2<f64>],
    target_points: &[Vector2<f64>],
) -> Vec<(usize, usize)> {
    let mut matches = Vec::with_capacity(source_points.len());

    for (i, source_point) in source_points.iter().enumerate() {
        let mut min_dist = f64::INFINITY;
        let mut closest_idx = 0;

        for (j, target_point) in target_points.iter().enumerate() {
            let dist = (source_point - target_point).norm_squared();

            if dist < min_dist {
                min_dist = dist;
                closest_idx = j;
            }
        }

        matches.push((i, closest_idx));
    }

    matches
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_exact_match() {
        let points = vec![
            Vector2::new(0.0, 0.0),
            Vector2::new(1.0, 0.0),
            Vector2::new(0.0, 1.0),
        ];
        let matches = find_closest_points(&points, &points);
        assert_eq!(matches, vec![(0, 0), (1, 1), (2, 2)]);
    }

    #[test]
    fn test_translated_points() {
        let source = vec![
            Vector2::new(0.0, 0.0),
            Vector2::new(1.0, 0.0),
            Vector2::new(0.0, 1.0),
        ];
        let target = vec![
            Vector2::new(0.1, 0.1),
            Vector2::new(1.1, 0.1),
            Vector2::new(0.1, 1.1),
        ];
        let matches = find_closest_points(&source, &target);
        assert_eq!(matches, vec![(0, 0), (1, 1), (2, 2)]);
    }

    #[test]
    fn test_asymmetric_sizes() {
        let source = vec![Vector2::new(0.0, 0.0), Vector2::new(10.0, 10.0)];
        let target = vec![
            Vector2::new(0.1, 0.0),
            Vector2::new(5.0, 5.0),
            Vector2::new(9.9, 10.0),
        ];
        let matches = find_closest_points(&source, &target);
        // source[0] closest to target[0], source[1] closest to target[2]
        assert_eq!(matches, vec![(0, 0), (1, 2)]);
    }

    #[test]
    fn test_many_to_one() {
        let source = vec![
            Vector2::new(0.0, 0.0),
            Vector2::new(0.1, 0.0),
            Vector2::new(0.2, 0.0),
        ];
        let target = vec![Vector2::new(0.0, 0.0), Vector2::new(100.0, 100.0)];
        let matches = find_closest_points(&source, &target);
        // All source points closest to target[0]
        assert_eq!(matches, vec![(0, 0), (1, 0), (2, 0)]);
    }
}

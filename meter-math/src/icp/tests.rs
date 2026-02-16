use super::*;
use approx::assert_relative_eq;
use nalgebra::{Matrix2, Vector2};
use rand::Rng;
use std::f64::consts::PI;

fn rotation_matrix(angle: f64) -> Matrix2<f64> {
    let cos_a = angle.cos();
    let sin_a = angle.sin();
    Matrix2::new(cos_a, -sin_a, sin_a, cos_a)
}

#[test]
fn test_icp_translation_only_full() {
    let translation = Vector2::new(2.0, 3.0);

    let mut many_source_points = Vec::new();
    let mut many_target_points = Vec::new();

    let grid_size: i32 = 5;
    for x in -grid_size..=grid_size {
        for y in -grid_size..=grid_size {
            let xf = x as f64 * (1.0 + 0.1 * (x as f64).abs());
            let yf = y as f64 * (1.0 + 0.2 * (y as f64).abs());

            many_source_points.push(xf);
            many_source_points.push(yf);

            many_target_points.push(xf + translation[0]);
            many_target_points.push(yf + translation[1]);
        }
    }

    let point_count = ((2 * grid_size + 1) * (2 * grid_size + 1)) as usize;

    let many_source =
        ndarray::Array2::from_shape_vec((point_count, 2), many_source_points).unwrap();
    let many_target =
        ndarray::Array2::from_shape_vec((point_count, 2), many_target_points).unwrap();

    let result = iterative_closest_point(&many_source, &many_target, 20, 1e-9).unwrap();
    assert!(!result.mean_squared_error.is_nan());
}

#[test]
fn test_icp_rotation_only_full() {
    let angle = PI / 6.0;
    let rotation = rotation_matrix(angle);

    let mut many_source_points = Vec::new();
    let mut many_target_points = Vec::new();

    let grid_size: i32 = 5;
    for x in -grid_size..=grid_size {
        for y in -grid_size..=grid_size {
            let xf = x as f64 * (1.0 + 0.1 * (x as f64).abs());
            let yf = y as f64 * (1.0 + 0.2 * (y as f64).abs());

            many_source_points.push(xf);
            many_source_points.push(yf);

            let p = Vector2::new(xf, yf);
            let p_rot = rotation * p;

            many_target_points.push(p_rot[0]);
            many_target_points.push(p_rot[1]);
        }
    }

    let point_count = ((2 * grid_size + 1) * (2 * grid_size + 1)) as usize;

    let many_source =
        ndarray::Array2::from_shape_vec((point_count, 2), many_source_points).unwrap();
    let many_target =
        ndarray::Array2::from_shape_vec((point_count, 2), many_target_points).unwrap();

    let result = iterative_closest_point(&many_source, &many_target, 20, 1e-9).unwrap();
    assert!(!result.mean_squared_error.is_nan());
}

#[test]
fn test_icp_rotation_and_translation_full() {
    let angle = PI / 4.0;
    let rotation = rotation_matrix(angle);
    let translation = Vector2::new(2.0, 1.0);

    let mut many_source_points = Vec::new();
    let mut many_target_points = Vec::new();

    let grid_size: i32 = 5;
    for x in -grid_size..=grid_size {
        for y in -grid_size..=grid_size {
            let xf = x as f64 * (1.0 + 0.1 * (x as f64).abs());
            let yf = y as f64 * (1.0 + 0.2 * (y as f64).abs());

            many_source_points.push(xf);
            many_source_points.push(yf);

            let p = Vector2::new(xf, yf);
            let p_transformed = rotation * p + translation;

            many_target_points.push(p_transformed[0]);
            many_target_points.push(p_transformed[1]);
        }
    }

    let point_count = ((2 * grid_size + 1) * (2 * grid_size + 1)) as usize;

    let many_source =
        ndarray::Array2::from_shape_vec((point_count, 2), many_source_points).unwrap();
    let many_target =
        ndarray::Array2::from_shape_vec((point_count, 2), many_target_points).unwrap();

    let result = iterative_closest_point(&many_source, &many_target, 20, 1e-9).unwrap();
    assert!(!result.mean_squared_error.is_nan());
}

#[test]
fn test_icp_with_noisy_data() {
    let mut rng = rand::rng();

    let angle = rng.random_range(0.0..2.0 * PI);
    let rotation = rotation_matrix(angle);
    let translation = Vector2::new(rng.random_range(-5.0..5.0), rng.random_range(-5.0..5.0));

    let mut source_points = Vec::new();
    let mut target_points = Vec::new();

    let grid_size: i32 = 5;
    let noise_level = 0.1;

    for x in -grid_size..=grid_size {
        for y in -grid_size..=grid_size {
            let xf = x as f64 * (1.0 + 0.1 * (x as f64).abs());
            let yf = y as f64 * (1.0 + 0.2 * (y as f64).abs());

            source_points.push(xf);
            source_points.push(yf);

            let p = Vector2::new(xf, yf);
            let p_transformed = rotation * p + translation;

            target_points.push(p_transformed[0] + noise_level * (rng.random::<f64>() - 0.5));
            target_points.push(p_transformed[1] + noise_level * (rng.random::<f64>() - 0.5));
        }
    }

    let point_count = ((2 * grid_size + 1) * (2 * grid_size + 1)) as usize;

    let source = ndarray::Array2::from_shape_vec((point_count, 2), source_points).unwrap();
    let target = ndarray::Array2::from_shape_vec((point_count, 2), target_points).unwrap();

    let result = iterative_closest_point(&source, &target, 50, 1e-9).unwrap();
    assert!(!result.mean_squared_error.is_nan());
    assert!(!result.mean_squared_error.is_infinite());
}

/// Simple struct implementing Locatable2d for testing icp_match_objects
#[derive(Debug, Clone, PartialEq)]
struct PointObject {
    id: usize,
    x_coord: f64,
    y_coord: f64,
}

impl Locatable2d for PointObject {
    fn x(&self) -> f64 {
        self.x_coord
    }
    fn y(&self) -> f64 {
        self.y_coord
    }
}

#[test]
fn test_icp_match_objects_identity() {
    let source_objs = vec![
        PointObject {
            id: 0,
            x_coord: 0.0,
            y_coord: 0.0,
        },
        PointObject {
            id: 1,
            x_coord: 1.0,
            y_coord: 0.0,
        },
        PointObject {
            id: 2,
            x_coord: 0.0,
            y_coord: 1.0,
        },
    ];
    let target_objs = source_objs.clone();

    let (matches, _icp_result) = icp_match_objects(&source_objs, &target_objs, 10, 1e-6)
        .expect("ICP should succeed with identical objects");

    assert_eq!(matches.len(), 3);
    for (src, tgt) in &matches {
        assert_eq!(src.id, tgt.id);
        assert_eq!(src.x_coord, tgt.x_coord);
        assert_eq!(src.y_coord, tgt.y_coord);
    }
}

#[test]
fn test_icp_match_objects_translation() {
    let source_objs = vec![
        PointObject {
            id: 0,
            x_coord: 0.0,
            y_coord: 0.0,
        },
        PointObject {
            id: 1,
            x_coord: 1.0,
            y_coord: 1.0,
        },
        PointObject {
            id: 2,
            x_coord: 0.5,
            y_coord: 5.0,
        },
    ];
    let translation = Vector2::new(5.0, -2.0);
    let target_objs: Vec<PointObject> = source_objs
        .iter()
        .map(|p| PointObject {
            id: p.id,
            x_coord: p.x_coord + translation.x,
            y_coord: p.y_coord + translation.y,
        })
        .collect();

    let (matches, _icp_result) = icp_match_objects(&source_objs, &target_objs, 20, 1e-6)
        .expect("ICP should succeed with translated objects");

    assert_eq!(matches.len(), 3);
    let mut matched_ids: Vec<(usize, usize)> = matches.iter().map(|(s, t)| (s.id, t.id)).collect();
    matched_ids.sort();
    assert_eq!(matched_ids, vec![(0, 0), (1, 1), (2, 2)]);
}

#[test]
fn test_icp_match_objects_rotation() {
    let source_objs = vec![
        PointObject {
            id: 0,
            x_coord: 0.0,
            y_coord: 0.0,
        },
        PointObject {
            id: 1,
            x_coord: 2.0,
            y_coord: 0.0,
        },
        PointObject {
            id: 2,
            x_coord: 0.0,
            y_coord: 1.0,
        },
    ];
    let angle = PI / 2.0 / 45.0; // 2 degrees
    let rotation = rotation_matrix(angle);
    let target_objs: Vec<PointObject> = source_objs
        .iter()
        .map(|p| {
            let point = Vector2::new(p.x_coord, p.y_coord);
            let rotated_point = rotation * point;
            PointObject {
                id: p.id,
                x_coord: rotated_point.x,
                y_coord: rotated_point.y,
            }
        })
        .collect();

    let (matches, _icp_result) = icp_match_objects(&source_objs, &target_objs, 20, 1e-6)
        .expect("ICP should succeed with rotated objects");

    assert_eq!(matches.len(), 3);
    let mut matched_ids: Vec<(usize, usize)> = matches.iter().map(|(s, t)| (s.id, t.id)).collect();
    matched_ids.sort();
    assert_eq!(matched_ids, vec![(0, 0), (1, 1), (2, 2)]);
}

#[test]
fn test_icp_match_objects_rotation_translation() {
    let source_objs = vec![
        PointObject {
            id: 0,
            x_coord: 1.0,
            y_coord: 1.0,
        },
        PointObject {
            id: 1,
            x_coord: 3.0,
            y_coord: 1.0,
        },
        PointObject {
            id: 2,
            x_coord: 1.0,
            y_coord: 2.0,
        },
    ];
    let angle = -PI / 2.0 / 45.0; // -2 degrees
    let rotation = rotation_matrix(angle);
    let translation = Vector2::new(-1.0, 0.0002);
    let target_objs: Vec<PointObject> = source_objs
        .iter()
        .map(|p| {
            let point = Vector2::new(p.x_coord, p.y_coord);
            let transformed_point = rotation * point + translation;
            PointObject {
                id: p.id,
                x_coord: transformed_point.x,
                y_coord: transformed_point.y,
            }
        })
        .collect();

    let (matches, _icp_result) = icp_match_objects(&source_objs, &target_objs, 30, 1e-6)
        .expect("ICP should succeed with rotated and translated objects");

    assert_eq!(matches.len(), 3);
    let mut matched_ids: Vec<(usize, usize)> = matches.iter().map(|(s, t)| (s.id, t.id)).collect();
    matched_ids.sort();
    assert_eq!(matched_ids, vec![(0, 0), (1, 1), (2, 2)]);
}

#[test]
fn test_icp_match_objects_empty_input() {
    let source_objs: Vec<PointObject> = vec![];
    let target_objs = vec![
        PointObject {
            id: 0,
            x_coord: 1.0,
            y_coord: 1.0,
        },
        PointObject {
            id: 1,
            x_coord: 2.0,
            y_coord: 2.0,
        },
    ];

    let matches_empty_source = icp_match_objects(&source_objs, &target_objs, 10, 1e-6);
    assert!(matches!(
        matches_empty_source,
        Err(ICPError::ArgumentError(_))
    ));

    let source_objs_non_empty = vec![
        PointObject {
            id: 0,
            x_coord: 1.0,
            y_coord: 1.0,
        },
        PointObject {
            id: 1,
            x_coord: 2.0,
            y_coord: 2.0,
        },
    ];
    let target_objs_empty: Vec<PointObject> = vec![];
    let result_empty_target =
        icp_match_objects(&source_objs_non_empty, &target_objs_empty, 10, 1e-6);
    assert!(matches!(
        result_empty_target,
        Err(ICPError::ArgumentError(_))
    ));

    let matches_both_empty = icp_match_objects(&source_objs, &target_objs_empty, 10, 1e-6);
    assert!(matches!(
        matches_both_empty,
        Err(ICPError::ArgumentError(_))
    ));
}

#[test]
fn test_icp_match_objects_different_sizes() {
    let source_objs = vec![
        PointObject {
            id: 0,
            x_coord: 0.0,
            y_coord: 0.0,
        },
        PointObject {
            id: 1,
            x_coord: 1.0,
            y_coord: 0.0,
        },
    ];
    let target_objs = vec![
        PointObject {
            id: 10,
            x_coord: 0.1,
            y_coord: 0.1,
        },
        PointObject {
            id: 11,
            x_coord: 1.1,
            y_coord: -0.1,
        },
        PointObject {
            id: 12,
            x_coord: 5.0,
            y_coord: 5.0,
        },
    ];

    let (matches, _icp_result) = icp_match_objects(&source_objs, &target_objs, 10, 1e-6)
        .expect("ICP should succeed with different sized sets");

    assert_eq!(matches.len(), 2);
    let mut matched_ids: Vec<(usize, usize)> = matches.iter().map(|(s, t)| (s.id, t.id)).collect();
    matched_ids.sort();
    assert_eq!(matched_ids, vec![(0, 10), (1, 11)]);
}

#[test]
fn test_icp_match_objects_argument_error() {
    let empty_source: Vec<PointObject> = vec![];
    let target_objs = vec![PointObject {
        id: 0,
        x_coord: 1.0,
        y_coord: 1.0,
    }];

    let result = icp_match_objects(&empty_source, &target_objs, 10, 1e-6);
    assert!(matches!(result, Err(ICPError::ArgumentError(_))));

    let source_objs = vec![PointObject {
        id: 0,
        x_coord: 1.0,
        y_coord: 1.0,
    }];
    let empty_target: Vec<PointObject> = vec![];
    let result = icp_match_objects(&source_objs, &empty_target, 10, 1e-6);
    assert!(matches!(result, Err(ICPError::ArgumentError(_))));

    let result = icp_match_objects(&source_objs, &target_objs, 10, -1e-6);
    assert!(matches!(result, Err(ICPError::ArgumentError(_))));

    let result = icp_match_objects(&source_objs, &target_objs, 10, 0.0);
    assert!(matches!(result, Err(ICPError::ArgumentError(_))));
}

#[test]
fn test_doctest_example() {
    use ndarray::Array2;

    let source = Array2::from_shape_vec((3, 2), vec![0.0, 0.0, 1.0, 0.0, 0.0, 1.0]).unwrap();
    let target = Array2::from_shape_vec((3, 2), vec![1.0, 1.0, 2.0, 1.0, 1.0, 2.0]).unwrap();

    let result = iterative_closest_point(&source, &target, 100, 1e-6).unwrap();

    assert_eq!(result.matches.len(), 3);
    assert!(result.mean_squared_error >= 0.0);
    assert!(result.iterations > 0 && result.iterations <= 100);
}

use crate::stats::{ks_critical_value, ks_test_normal, pearson_correlation};
use rand::{rngs::StdRng, SeedableRng};
use rand_distr::{Distribution, Normal};

const DISABLE_TRANSFORM: bool = true;

#[test]
fn test_icp_residual_normality() {
    let n_points = 400;
    let n_trials = 50;
    let noise_std = 1.0;
    let seed = 42;

    let _rng = StdRng::seed_from_u64(seed);
    let noise_dist = Normal::new(0.0, noise_std).unwrap();

    let mut all_residuals_x = Vec::new();
    let mut all_residuals_y = Vec::new();

    for trial in 0..n_trials {
        let mut source_points = Vec::new();
        let mut target_points = Vec::new();

        let trial_seed = seed + trial as u64;
        let mut trial_rng = StdRng::seed_from_u64(trial_seed);

        let mut source_vec = Vec::new();
        for _ in 0..n_points {
            let x = trial_rng.random_range(-0.0..8000.0);
            let y = trial_rng.random_range(-0.0..2000.0);
            source_points.push(x);
            source_points.push(y);
            source_vec.push(Vector2::new(x, y));
        }

        let true_angle = if DISABLE_TRANSFORM {
            0.0
        } else {
            trial_rng.random_range(-PI / 4.0..PI / 4.0)
        };
        let true_translation = if DISABLE_TRANSFORM {
            Vector2::new(0.0, 0.0)
        } else {
            Vector2::new(
                trial_rng.random_range(-2.0..2.0),
                trial_rng.random_range(-2.0..2.0),
            )
        };

        let cos_a = true_angle.cos();
        let sin_a = true_angle.sin();
        let true_rotation = Matrix2::new(cos_a, -sin_a, sin_a, cos_a);

        for i in 0..n_points {
            let source_point = source_vec[i];
            let transformed = true_rotation * source_point + true_translation;

            let noise_x = noise_dist.sample(&mut trial_rng);
            let noise_y = noise_dist.sample(&mut trial_rng);

            target_points.push(transformed.x + noise_x);
            target_points.push(transformed.y + noise_y);
        }

        let source_array = ndarray::Array2::from_shape_vec((n_points, 2), source_points).unwrap();
        let target_array = ndarray::Array2::from_shape_vec((n_points, 2), target_points).unwrap();

        let icp_result = iterative_closest_point(&source_array, &target_array, 100, 1e-9).unwrap();

        for &(src_idx, tgt_idx) in &icp_result.matches {
            let source_point = Vector2::new(source_array[(src_idx, 0)], source_array[(src_idx, 1)]);
            let target_point = Vector2::new(target_array[(tgt_idx, 0)], target_array[(tgt_idx, 1)]);

            let transformed = icp_result.rotation * source_point + icp_result.translation;

            let residual_x = transformed.x - target_point.x;
            let residual_y = transformed.y - target_point.y;

            all_residuals_x.push(residual_x);
            all_residuals_y.push(residual_y);
        }
    }

    let ks_statistic_x = ks_test_normal(&all_residuals_x);
    let ks_statistic_y = ks_test_normal(&all_residuals_y);

    let mean_x: f64 = all_residuals_x.iter().sum::<f64>() / all_residuals_x.len() as f64;
    let mean_y: f64 = all_residuals_y.iter().sum::<f64>() / all_residuals_y.len() as f64;

    let std_x: f64 = (all_residuals_x
        .iter()
        .map(|x| (x - mean_x).powi(2))
        .sum::<f64>()
        / all_residuals_x.len() as f64)
        .sqrt();
    let std_y: f64 = (all_residuals_y
        .iter()
        .map(|y| (y - mean_y).powi(2))
        .sum::<f64>()
        / all_residuals_y.len() as f64)
        .sqrt();

    let _critical_value = ks_critical_value(all_residuals_x.len(), 0.05);

    assert_relative_eq!(mean_x, 0.0, epsilon = noise_std * 3.0);
    assert_relative_eq!(mean_y, 0.0, epsilon = noise_std * 3.0);

    assert_relative_eq!(std_x, noise_std, epsilon = noise_std * 0.5);
    assert_relative_eq!(std_y, noise_std, epsilon = noise_std * 0.5);

    let ks_threshold = 0.05;
    assert!(
        ks_statistic_x < ks_threshold,
        "X residuals fail normality test: KS = {ks_statistic_x:.6} > {ks_threshold:.6}"
    );
    assert!(
        ks_statistic_y < ks_threshold,
        "Y residuals fail normality test: KS = {ks_statistic_y:.6} > {ks_threshold:.6}"
    );

    let correlation = pearson_correlation(&all_residuals_x, &all_residuals_y);
    assert_relative_eq!(correlation, 0.0, epsilon = 0.1);
}

#[test]
fn test_icp_with_outliers() {
    let n_points = 50;
    let n_outliers = 5;
    let seed = 123;

    let mut rng = StdRng::seed_from_u64(seed);

    let mut source_points = Vec::new();
    for _ in 0..(n_points + n_outliers) {
        let x = rng.random_range(-5.0..5.0);
        let y = rng.random_range(-5.0..5.0);
        source_points.push(x);
        source_points.push(y);
    }

    let true_angle = PI / 8.0;
    let true_translation = Vector2::new(1.5, -0.5);
    let cos_a = true_angle.cos();
    let sin_a = true_angle.sin();
    let true_rotation = Matrix2::new(cos_a, -sin_a, sin_a, cos_a);

    let mut target_points = Vec::new();

    for i in 0..n_points {
        let x = source_points[i * 2];
        let y = source_points[i * 2 + 1];
        let source_point = Vector2::new(x, y);
        let transformed = true_rotation * source_point + true_translation;
        target_points.push(transformed.x);
        target_points.push(transformed.y);
    }

    for _ in 0..n_outliers {
        target_points.push(rng.random_range(-20.0..20.0));
        target_points.push(rng.random_range(-20.0..20.0));
    }

    let total_points = n_points + n_outliers;
    let source_array = ndarray::Array2::from_shape_vec((total_points, 2), source_points).unwrap();
    let target_array = ndarray::Array2::from_shape_vec((total_points, 2), target_points).unwrap();

    let icp_result = iterative_closest_point(&source_array, &target_array, 100, 1e-6).unwrap();

    assert!(!icp_result.mean_squared_error.is_nan());
    assert!(!icp_result.mean_squared_error.is_infinite());
}

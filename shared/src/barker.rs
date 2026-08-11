/// Barker codes for synchronization and timing patterns
/// These are binary sequences with good autocorrelation properties
///
/// Barker-11 sequence: [1, 1, 1, -1, -1, -1, 1, -1, -1, 1, -1]
/// Represented as bits where 1 = high, 0 = low
pub const BARKER_11: [bool; 11] = [
    true, true, true, false, false, false, true, false, false, true, false,
];

/// Barker-13 sequence: [1, 1, 1, 1, 1, -1, -1, 1, 1, -1, 1, -1, 1]
pub const BARKER_13: [bool; 13] = [
    true, true, true, true, true, false, false, true, true, false, true, false, true,
];

/// Barker-7 sequence: [1, 1, 1, -1, -1, 1, -1]
pub const BARKER_7: [bool; 7] = [true, true, true, false, false, true, false];

/// Barker-5 sequence: [1, 1, 1, -1, 1]
pub const BARKER_5: [bool; 5] = [true, true, true, false, true];

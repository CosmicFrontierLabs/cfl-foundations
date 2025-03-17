//! Hardware module for telescope and sensor configurations

pub mod sensor;
pub mod star_projection;
pub mod telescope;

pub use sensor::SensorConfig;
pub use star_projection::field_diameter;
pub use telescope::TelescopeConfig;

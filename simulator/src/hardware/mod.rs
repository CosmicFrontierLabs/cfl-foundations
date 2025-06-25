//! Hardware module for telescope and sensor configurations

pub mod dark_current;
pub mod satellite;
pub mod sensor;
pub mod star_projection;
pub mod telescope;

pub use satellite::SatelliteConfig;
pub use sensor::SensorConfig;
pub use star_projection::field_diameter;
pub use telescope::TelescopeConfig;

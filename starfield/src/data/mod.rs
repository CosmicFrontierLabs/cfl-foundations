//! Data module for downloading and managing astronomical data
//!
//! This module provides functionality for downloading, caching, and loading
//! astronomical datasets like star catalogs.

mod downloader;
mod gaia_downloader;

pub use downloader::{download_hipparcos, get_cache_dir, ensure_cache_dir};
pub use gaia_downloader::{download_gaia_catalog, get_gaia_cache_dir, ensure_gaia_cache_dir, 
                          list_cached_gaia_files, download_gaia_file};
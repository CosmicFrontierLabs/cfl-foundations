use once_cell::sync::Lazy;
use std::env;
use std::path::PathBuf;

#[derive(thiserror::Error, Debug)]
pub enum TestUtilError {
    #[error("Failed to find project root: {0}")]
    ProjectRootNotFound(String),
}

pub fn find_project_root() -> Result<PathBuf, TestUtilError> {
    let mut current_dir = env::current_dir().map_err(|e| {
        TestUtilError::ProjectRootNotFound(format!("Failed to get current directory: {e}"))
    })?;

    loop {
        let cargo_toml = current_dir.join("Cargo.toml");
        if cargo_toml.exists() {
            let content = std::fs::read_to_string(&cargo_toml).map_err(|e| {
                TestUtilError::ProjectRootNotFound(format!("Failed to read Cargo.toml: {e}"))
            })?;

            if content.contains("[workspace]") {
                return Ok(current_dir);
            }
        }

        if !current_dir.pop() {
            break;
        }
    }

    Err(TestUtilError::ProjectRootNotFound(
        "Workspace root not found".to_string(),
    ))
}

static PROJECT_ROOT: Lazy<PathBuf> =
    Lazy::new(|| find_project_root().expect("Failed to find project root directory"));

pub fn get_output_dir() -> PathBuf {
    let output_dir = PROJECT_ROOT.join("test_output");

    if !output_dir.exists() {
        std::fs::create_dir_all(&output_dir).expect("Failed to create output directory");
    }

    output_dir
}

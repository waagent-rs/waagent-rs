use std::fs::{self, File};
use std::io::{self, Read};
use std::path::{Path, PathBuf};

use glob::glob;
use tracing::{debug, warn};

pub fn read_file(path: &Path) -> std::io::Result<String> {
    // Errors should be propagated since this is a lib, caller in waagent-rs should handle errors.
    // https://doc.rust-lang.org/book/ch09-02-recoverable-errors-with-result.html#propagating-errors
    validate_path(path)?;
    let mut file = File::open(path)?;
    let mut buffer = String::new();

    file.read_to_string(&mut buffer)?;
    Ok(buffer)
}

/// Remove the given files. Each entry may be a literal path or a glob pattern
/// (e.g. `/var/lib/waagent/GoalState.*.xml`). Missing files are ignored to
/// mirror the behavior of WALinuxAgent's `fileutil.rm_files`.
pub fn rm_files(paths: &[impl AsRef<str>]) {
    for pattern in paths {
        let pattern = pattern.as_ref();
        for entry in expand_glob(pattern) {
            if entry.is_file() || entry.is_symlink() {
                if let Err(err) = fs::remove_file(&entry) {
                    warn!("failed to remove file {}: {}", entry.display(), err);
                } else {
                    debug!("removed file {}", entry.display());
                }
            }
        }
    }
}

/// Recursively remove the given directories. Missing directories are ignored.
pub fn rm_dirs(paths: &[impl AsRef<str>]) {
    for pattern in paths {
        let pattern = pattern.as_ref();
        for entry in expand_glob(pattern) {
            if entry.is_dir() {
                if let Err(err) = fs::remove_dir_all(&entry) {
                    warn!("failed to remove directory {}: {}", entry.display(), err);
                } else {
                    debug!("removed directory {}", entry.display());
                }
            }
        }
    }
}

fn expand_glob(pattern: &str) -> Vec<PathBuf> {
    if pattern.contains(['*', '?', '[']) {
        match glob(pattern) {
            Ok(paths) => paths.filter_map(|p| p.ok()).collect(),
            Err(err) => {
                warn!("invalid glob pattern '{}': {}", pattern, err);
                Vec::new()
            }
        }
    } else {
        let p = PathBuf::from(pattern);
        if p.exists() {
            vec![p]
        } else {
            Vec::new()
        }
    }
}

fn validate_path(path: &Path) -> io::Result<()> {
    // Ensure the path exists and points to a regular file before attempting to open it.
    let metadata = fs::metadata(path).map_err(|err| match err.kind() {
        io::ErrorKind::NotFound => err,
        _ => io::Error::new(
            io::ErrorKind::InvalidInput,
            format!("Unable to access file at '{}': {}", path.display(), err),
        ),
    })?;

    if !metadata.is_file() {
        return Err(io::Error::new(
            io::ErrorKind::InvalidInput,
            format!("Path '{}' is not a regular file", path.display()),
        ));
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use std::io;

    #[test]
    fn rejects_directory_paths() {
        let temp_dir = std::env::temp_dir().join(format!(
            "waagent_core_test_dir_{}",
            std::process::id()
        ));
        fs::create_dir_all(&temp_dir).expect("failed to create temp directory");

        let result = read_file(&temp_dir);
        assert!(result.is_err());
        assert_eq!(result.unwrap_err().kind(), io::ErrorKind::InvalidInput);

        fs::remove_dir_all(&temp_dir).expect("failed to clean up temp directory");
    }
}

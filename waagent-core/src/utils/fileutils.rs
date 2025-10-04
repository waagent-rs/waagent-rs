use std::fs::{self, File};
use std::io::{self, Read};
use std::path::Path;

pub fn read_file(path: &Path) -> std::io::Result<String> {
    // Errors should be propagated since this is a lib, caller in waagent-rs should handle errors.
    // https://doc.rust-lang.org/book/ch09-02-recoverable-errors-with-result.html#propagating-errors
    validate_path(path)?;
    let mut file = File::open(path)?;
    let mut buffer = String::new();

    file.read_to_string(&mut buffer)?;
    Ok(buffer)
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

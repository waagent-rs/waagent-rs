use std::fs::File;
use std::io::prelude::*;
use std::path::Path;

pub fn read_file(path: &Path) -> std::io::Result<String> {
    // Errors should be propagated since this is a lib, caller in waagent-rs should handle errors.
    // https://doc.rust-lang.org/book/ch09-02-recoverable-errors-with-result.html#propagating-errors
    let mut file = File::open(path)?;
    let mut buffer = String::new();

    file.read_to_string(&mut buffer)?;
    Ok(buffer)
}

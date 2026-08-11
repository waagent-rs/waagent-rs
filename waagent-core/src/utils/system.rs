use std::process::Command;

type Result<T> = std::result::Result<T, Box<dyn std::error::Error>>;

/// Get the UID of a specific user
pub fn get_user_uid(username: &str) -> Result<String> {
    let output = Command::new("id")
        .args(["-u", username])
        .output()
        .map_err(|e| format!("Failed to execute id command: {}", e))?;

    if output.status.success() {
        let uid = String::from_utf8_lossy(&output.stdout).trim().to_string();
        Ok(uid)
    } else {
        let stderr = String::from_utf8_lossy(&output.stderr);
        Err(format!("Failed to get uid for user {}: {}", username, stderr).into())
    }
}

use std::fs;
use std::path::Path;

pub fn replace_file(temporary: &Path, destination: &Path) -> Result<(), String> {
    #[cfg(target_os = "windows")]
    if destination.exists() {
        fs::remove_file(destination).map_err(|error| error.to_string())?;
    }
    fs::rename(temporary, destination).map_err(|error| error.to_string())
}

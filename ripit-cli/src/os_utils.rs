use std::path::PathBuf;

// TODO consider using an environment variable to specify the location
// (allow the user to force a specific location)
pub fn find_makemkvcon() -> PathBuf {
    // Determine the binary name and fallback path based on OS and architecture
    let (binary_name, fallback_path) = if cfg!(target_os = "linux") {
        ("makemkvcon", None)
    } else if cfg!(target_os = "windows") {
        if cfg!(target_pointer_width = "64") {
            ("makemkvcon64.exe", Some("C:\\Program Files (x86)\\MakeMKV"))
        } else {
            ("makemkvcon.exe", Some("C:\\Program Files\\MakeMKV"))
        }
    } else {
        // Unsupported OS (e.g. MacOS/Darwin)
        // -> return binary name as fallback
        return PathBuf::from("makemkvcon");
    };

    // First, try to find the binary in PATH
    if let Ok(path) = which::which(binary_name) {
        return path;
    }

    // If not found in PATH and we have a fallback path (Windows), try there
    if let Some(fallback) = fallback_path {
        let full_path = PathBuf::from(fallback).join(binary_name);
        if full_path.exists() {
            return full_path;
        }
    }

    // As last resort, return just the binary name and let the system handle it
    PathBuf::from(binary_name)
}

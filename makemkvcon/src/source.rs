/*
    source definition (as used by makemkvcon)
*/

use std::fmt;
use std::path::PathBuf;

#[allow(unused_imports)]
use log::{debug, error, info, warn};

use regex::Regex;

#[derive(Clone, Debug, PartialEq, serde::Serialize)]
pub enum Source {
    DriveId(u8),
    DriveLetter(String),
    DeviceName(PathBuf),
    IsoFile(PathBuf),
    Directory(PathBuf),
}

pub fn parse_source(source: &str) -> Option<Source> {
    let rx_drive_id = Regex::new(r"^(\d)$").unwrap();
    let rx_drive_letter = Regex::new(r"^[A-Z]:\\?$").unwrap();
    let rx_device_name = Regex::new(r"^(/dev/.*)$").unwrap();

    if rx_drive_id.is_match(source) {
        // source is an optical drive id
        Some(Source::DriveId(source.parse().unwrap()))
    } else if rx_drive_letter.is_match(source) {
        // source is a drive letter (Windows)
        Some(Source::DriveLetter(source.to_string()))
    } else if rx_device_name.is_match(source) {
        // source is a device name (Linux)
        Some(Source::DeviceName(source.into()))
    } else {
        let source_path = PathBuf::from(&source);
        if source_path.is_dir() {
            // source is a directory
            Some(Source::IsoFile(source.into()))
        } else if source_path.is_file() {
            // source is a file (assume iso)
            Some(Source::IsoFile(source.into()))
        } else {
            error!(
                "Usage error: Source must be a disc id, drive letter, device name, iso file, or directory!"
            );
            None
        }
    }
}

impl fmt::Display for Source {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Source::DriveId(id) => write!(f, "disc:{}", id),
            Source::DriveLetter(letter) => write!(f, "dev:{}", letter),
            Source::DeviceName(path) => write!(f, "dev:{}", path.to_string_lossy()),
            Source::IsoFile(path) => write!(f, "iso:{}", path.to_string_lossy()),
            Source::Directory(path) => write!(f, "file:{}", path.to_string_lossy()),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::PathBuf;

    #[test]
    fn test_format_drive_id() {
        // ----------------------------------------------------------------
        let computed = Source::DriveId(0).to_string();
        let expected = "disc:0".to_string();
        // ----------------------------------------------------------------
        assert_eq!(computed, expected);
    }

    #[test]
    fn test_format_drive_letter1() {
        // ----------------------------------------------------------------
        let computed = Source::DriveLetter("E:".to_string()).to_string();
        let expected = "dev:E:".to_string();
        // ----------------------------------------------------------------
        assert_eq!(computed, expected);
    }

    #[test]
    fn test_format_drive_letter2() {
        // ----------------------------------------------------------------
        let computed = Source::DriveLetter(r"E:\".to_string()).to_string();
        let expected = r"dev:E:\".to_string();
        // ----------------------------------------------------------------
        assert_eq!(computed, expected);
    }

    #[test]
    fn test_format_device_name() {
        // ----------------------------------------------------------------
        let computed = Source::DeviceName(PathBuf::from("/dev/sr0")).to_string();
        let expected = "dev:/dev/sr0".to_string();
        // ----------------------------------------------------------------
        assert_eq!(computed, expected);
    }

    #[test]
    fn test_format_iso_file() {
        // ----------------------------------------------------------------
        let computed = Source::IsoFile(PathBuf::from("/home/user/dvd.iso")).to_string();
        let expected = "iso:/home/user/dvd.iso".to_string();
        // ----------------------------------------------------------------
        assert_eq!(computed, expected);
    }

    #[test]
    fn test_format_directory() {
        // ----------------------------------------------------------------
        let computed = Source::Directory(PathBuf::from("/home/user/dvd_folder")).to_string();
        let expected = "file:/home/user/dvd_folder".to_string();
        // ----------------------------------------------------------------
        assert_eq!(computed, expected);
    }

    #[test]
    fn test_parse_source_as_disc() {
        // ----------------------------------------------------------------
        let computed = parse_source("1").unwrap();
        let expected = Source::DriveId(1);
        // ----------------------------------------------------------------
        assert_eq!(computed, expected);
    }

    #[test]
    fn test_parse_source_as_drive_letter() {
        // ----------------------------------------------------------------
        let computed = parse_source("E:").unwrap();
        let expected = Source::DriveLetter("E:".to_string());
        // ----------------------------------------------------------------
        assert_eq!(computed, expected);
    }

    #[test]
    fn test_parse_source_as_device_name() {
        // ----------------------------------------------------------------
        let computed = parse_source("/dev/sr1").unwrap();
        let expected = Source::DeviceName(PathBuf::from("/dev/sr1"));
        // ----------------------------------------------------------------
        assert_eq!(computed, expected);
    }

    #[test]
    fn test_parse_source_as_directory() {
        // ----------------------------------------------------------------
        let computed = parse_source(".").unwrap();
        let expected = Source::Directory(PathBuf::from("."));
        // ----------------------------------------------------------------
        assert_eq!(computed, expected);
    }

    #[test]
    fn test_parse_source_as_iso_file() {
        // ----------------------------------------------------------------
        // the file must exist, does not verify if the file is an actual
        // ISO file
        let computed = parse_source("src/lib.rs").unwrap();
        let expected = Source::IsoFile(PathBuf::from("src/lib.rs"));
        // ----------------------------------------------------------------
        assert_eq!(computed, expected);
    }

    #[test]
    fn test_parse_source_failure() {
        // ----------------------------------------------------------------
        let computed = parse_source("does_not_exist.iso");
        let expected = None;
        // ----------------------------------------------------------------
        assert_eq!(computed, expected);
    }
}

/*
    source definition (as used by makemkvcon)

can be any of:

- 'iso:<FileName>' - disc image (ISO file)
- 'file:<FolderName>' - filename or directory
- 'disc:<DiscId>' - disc with id
- 'dev:<DeviceName>' - device name or drive letter
*/

use std::fmt;
use std::path::PathBuf;

#[allow(unused_imports)]
use log::{debug, error, info, warn};

use regex::Regex;

#[derive(Clone, Debug, PartialEq, serde::Serialize)]
pub enum Source {
    DiscId(u8),
    DriveLetter(String), // Windows
    DeviceName(PathBuf), // Linux, MacOS
    IsoFile(PathBuf),
    Directory(PathBuf),
}

/// auto-detect the source's type using the user-provided value, e.g.:
/// - '1' -> disc:1 (DiscId)
/// - 'E:' -> dev:E: (DriveLetter)
/// - 'image.iso' -> file://image.iso (IsoFile)
///
/// (returns 'none' if unable to determine the source type)
pub fn parse_source(source: &str) -> Option<Source> {
    let rx_disc_id = Regex::new(r"^(\d|1[0-5])$").unwrap();
    let rx_drive_letter = Regex::new(r"^([A-Za-z]:)\\?$").unwrap();
    let rx_device_name = Regex::new(r"^(/dev/.*)$").unwrap();

    if rx_disc_id.is_match(source) {
        // source is an optical drive id
        Some(Source::DiscId(source.parse().unwrap()))
    } else if let Some(matched) = rx_drive_letter.captures(source) {
        // source is a drive letter (Windows)
        // -> extract the drive letter (without backslash)
        let drive_letter = matched.get(1).unwrap().as_str().to_uppercase();
        Some(Source::DriveLetter(drive_letter))
    } else if rx_device_name.is_match(source) {
        // source is a device name (Linux or MacOS)
        Some(Source::DeviceName(source.into()))
    } else {
        let source_path = PathBuf::from(&source);
        if source_path.is_dir() {
            // source is a directory
            Some(Source::Directory(source.into()))
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
            Source::DiscId(id) => write!(f, "disc:{}", id),
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
    fn test_format_disc_id() {
        // ----------------------------------------------------------------
        let computed = Source::DiscId(0).to_string();
        let expected = "disc:0".to_string();
        // ----------------------------------------------------------------
        assert_eq!(computed, expected);
    }

    #[test]
    fn test_format_drive_letter_1() {
        // ----------------------------------------------------------------
        let computed = Source::DriveLetter("E:".to_string()).to_string();
        let expected = "dev:E:".to_string();
        // ----------------------------------------------------------------
        assert_eq!(computed, expected);
    }

    #[test]
    fn test_format_drive_letter_2() {
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
    fn test_parse_source_as_disc_1() {
        // ----------------------------------------------------------------
        let computed = parse_source("0").unwrap();
        let expected = Source::DiscId(0);
        // ----------------------------------------------------------------
        assert_eq!(computed, expected);
    }

    #[test]
    fn test_parse_source_as_disc_2() {
        // ----------------------------------------------------------------
        let computed = parse_source("15").unwrap();
        let expected = Source::DiscId(15);
        // ----------------------------------------------------------------
        assert_eq!(computed, expected);
    }

    // MakeMkv supports up to 15 discs
    #[test]
    fn test_parse_source_as_disc_oor() {
        // ----------------------------------------------------------------
        let computed = parse_source("16");
        let expected = None;
        // ----------------------------------------------------------------
        assert_eq!(computed, expected);
    }

    #[test]
    fn test_parse_source_as_drive_letter_lc1() {
        // ----------------------------------------------------------------
        let computed = parse_source(r"a:").unwrap();
        let expected = Source::DriveLetter("A:".to_string());
        // ----------------------------------------------------------------
        assert_eq!(computed, expected);
    }

    #[test]
    fn test_parse_source_as_drive_letter_lc2() {
        // ----------------------------------------------------------------
        let computed = parse_source(r"z:\").unwrap();
        let expected = Source::DriveLetter("Z:".to_string());
        // ----------------------------------------------------------------
        assert_eq!(computed, expected);
    }

    #[test]
    fn test_parse_source_as_drive_letter_uc1() {
        // ----------------------------------------------------------------
        let computed = parse_source(r"A:").unwrap();
        let expected = Source::DriveLetter("A:".to_string());
        // ----------------------------------------------------------------
        assert_eq!(computed, expected);
    }

    #[test]
    fn test_parse_source_as_drive_letter_uc2() {
        // ----------------------------------------------------------------
        let computed = parse_source(r"Z:\").unwrap();
        let expected = Source::DriveLetter("Z:".to_string());
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
    fn test_parse_source_as_device_name_alias1() {
        // ----------------------------------------------------------------
        let computed = parse_source("/dev/cdrom").unwrap();
        let expected = Source::DeviceName(PathBuf::from("/dev/cdrom"));
        // ----------------------------------------------------------------
        assert_eq!(computed, expected);
    }

    #[test]
    fn test_parse_source_as_device_name_alias2() {
        // ----------------------------------------------------------------
        let computed = parse_source("/dev/dvd").unwrap();
        let expected = Source::DeviceName(PathBuf::from("/dev/dvd"));
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
    fn test_fail_to_parse_empty_source() {
        // ----------------------------------------------------------------
        let computed = parse_source("");
        let expected = None;
        // ----------------------------------------------------------------
        assert_eq!(computed, expected);
    }

    #[test]
    fn test_fail_to_parse_missing_file() {
        // ----------------------------------------------------------------
        let computed = parse_source("does_not_exist.iso");
        let expected = None;
        // ----------------------------------------------------------------
        assert_eq!(computed, expected);
    }

    #[test]
    fn test_fail_to_parse_leading_whitespace() {
        // ----------------------------------------------------------------
        let computed = parse_source(" E:");
        let expected = None;
        // ----------------------------------------------------------------
        assert_eq!(computed, expected);
    }

    #[test]
    fn test_fail_to_parse_trailing_whitespace() {
        // ----------------------------------------------------------------
        let computed = parse_source("E: ");
        let expected = None;
        // ----------------------------------------------------------------
        assert_eq!(computed, expected);
    }
}

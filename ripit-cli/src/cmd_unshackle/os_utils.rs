/*
    create a unique volume id (OS-specific)

    The disc name can be something sensible like "BLADE TRINITY UNRATED CUT"
    or something completely useless like "DVDVolume" or "LOGICAL_VOLUME_ID".
    Generic names would be especially nasty if encountered on a TV show
    spanning multiple discs.

    To deal with this situation, we augment the name with a disc-specific
    (hopefully unique) value, e.g. "DVDVolume" becomes "DVDVolume_29615A81".
*/

#[allow(unused_imports)]
use log::{debug, error, info, warn};

use regex::Regex;

#[allow(dead_code)]
#[cfg(target_os = "windows")]
pub struct VolumeInfo {
    pub volume_name: String,
    pub volume_serial: String,
    pub max_component_length: u32,
    pub filesystem_flags: u32,
    pub filesystem_name: String,
}

// Windows-specific - example output:
// ------------------------------------------------------------------------
// Volume Name: DVDVolume
// Volume Serial Number: 29615A81
// Maximum Component Length: 254
// File System Flags: 1480207
// File System Name: UDF
// ------------------------------------------------------------------------
#[cfg(target_os = "windows")]
pub fn get_volume_info(path: &str) -> Option<VolumeInfo> {
    use winapi::shared::minwindef::BOOL;
    use winapi::um::fileapi::GetVolumeInformationW;

    let mut volume_name = vec![0u16; 256];
    let mut file_system_name = vec![0u16; 256];
    let mut volume_serial: u32 = 0;
    let mut max_component_length: u32 = 0;
    let mut flags: u32 = 0;

    let path_wide: Vec<u16> = path.encode_utf16().chain(std::iter::once(0)).collect();

    let result: BOOL = unsafe {
        GetVolumeInformationW(
            path_wide.as_ptr(),
            volume_name.as_mut_ptr(),
            volume_name.len() as u32,
            &mut volume_serial,
            &mut max_component_length,
            &mut flags,
            file_system_name.as_mut_ptr(),
            file_system_name.len() as u32,
        )
    };

    if result != 0 {
        let volume_name_str = String::from_utf16_lossy(
            &volume_name[..volume_name
                .iter()
                .position(|&c| c == 0)
                .unwrap_or(volume_name.len())],
        );
        let file_system_name_str = String::from_utf16_lossy(
            &file_system_name[..file_system_name
                .iter()
                .position(|&c| c == 0)
                .unwrap_or(file_system_name.len())],
        );
        Some(VolumeInfo {
            volume_name: volume_name_str,
            volume_serial: format!("{:X}", volume_serial),
            max_component_length,
            filesystem_flags: flags,
            filesystem_name: file_system_name_str,
        })
    } else {
        // failed to get volume information
        None
    }
}

#[allow(dead_code)]
#[cfg(target_os = "linux")]
pub struct BlockId {
    pub dev_name: String,  // /dev/sr0
    pub uuid: String,      // bf457f1a20202020
    pub label: String,     // DVDVolume
    pub block_size: usize, // 2048
    pub fs_type: String,   // udf
}

// LINUX-specific - example output:
// ------------------------------------------------------------------------
// DEVNAME=/dev/sr0
// UUID=bf457f1a20202020
// LABEL=DVDVolume
// BLOCK_SIZE=2048
// TYPE=udf
// ------------------------------------------------------------------------
#[cfg(target_os = "linux")]
pub fn get_blkid(path: &str) -> Option<BlockId> {
    use std::process::Command;

    let output = Command::new("blkid")
        .args(["--output=export", path])
        .output()
        .ok()?;

    if !output.status.success() {
        return None;
    }

    let stdout = String::from_utf8(output.stdout).ok()?;

    if stdout.trim().is_empty() {
        return None;
    }

    let mut dev_name = None;
    let mut uuid = None;
    let mut label = None;
    let mut block_size = None;
    let mut fs_type = None;

    for line in stdout.lines() {
        if let Some((key, value)) = line.split_once('=') {
            match key {
                "DEVNAME" => dev_name = Some(value.to_string()),
                "UUID" => uuid = Some(value.to_string()),
                "LABEL" => label = Some(value.replace(r"\ ", " ")),
                "BLOCK_SIZE" => block_size = value.parse().ok(),
                "TYPE" => fs_type = Some(value.to_string()),
                _ => {}
            }
        }
    }

    Some(BlockId {
        dev_name: dev_name?,
        uuid: uuid?,
        label: label?,
        block_size: block_size?,
        fs_type: fs_type?,
    })
}

pub fn get_volume_id(path: &str) -> Option<String> {
    #[cfg(target_os = "linux")]
    {
        if let Some(block_id) = get_blkid(path) {
            Some(format!("{}_{}", block_id.label, block_id.uuid))
        } else {
            None
        }
    }

    #[cfg(target_os = "windows")]
    {
        if let Some(volume_info) = get_volume_info(path) {
            Some(format!(
                "{}_{}",
                volume_info.volume_name, volume_info.volume_serial
            ))
        } else {
            None
        }
    }

    #[cfg(target_os = "macos")]
    {
        // not implemented
        None
    }

    #[cfg(not(any(target_os = "linux", target_os = "windows", target_os = "macos")))]
    {
        None
    }
}

#[cfg(target_os = "linux")]
fn is_device_name(source: &str) -> bool {
    let rx_device: Regex = Regex::new(r"^/dev/").unwrap();
    rx_device.is_match(source)
}

#[cfg(target_os = "linux")]
fn eject_medium_linux(source: &std::path::Path) {
    let source_str = source.to_string_lossy();
    // TODO consider testing for block device instead of '/dev/'
    if is_device_name(&source_str) {
        let result = std::process::Command::new("eject")
            .arg(source)
            .stdout(std::process::Stdio::null())
            .stderr(std::process::Stdio::null())
            .status();

        match result {
            Ok(status) => {
                if status.success() {
                    info!("Ejected medium from '{}' using eject.", &source_str);
                } else {
                    warn!("Linux eject command returned status {:?}.", status.code());
                }
            }
            Err(error) => {
                warn!("Failed to execute eject command: {}", error);
            }
        }
    } else {
        warn!("Unable to eject, source '{}' is not a device!", &source_str);
    };
}

#[cfg(target_os = "windows")]
fn is_drive_letter(source: &str) -> bool {
    // rules for DOS/Windows:
    // - drive letter A: and B: are reserved for floppy drives
    // - drive letter C: is assigned to the first hard disk drive partition
    // - drive letter D: to Z: could be optical drives
    // - technically it's possible to have weird drive letters like '1:'
    //   but that requires SUBST-trickery and we ignore it
    let rx_drive: Regex = Regex::new(r"^[A-Z]:$").unwrap();
    rx_drive.is_match(source)
}

#[cfg(target_os = "windows")]
fn eject_medium_windows(source: &std::path::Path) {
    let source_str = source.to_string_lossy();
    if is_drive_letter(&source_str) {
        let ps_command = format!(
            "$driveEject = New-Object -comObject Shell.Application; $driveEject.Namespace(17).ParseName(\"{}\").InvokeVerb(\"Eject\")",
            source_str
        );
        let result = std::process::Command::new("powershell.exe")
            .args(["-NoProfile", "-Command", &ps_command])
            .stdout(std::process::Stdio::null())
            .stderr(std::process::Stdio::null())
            .status();

        match result {
            Ok(status) => {
                if status.success() {
                    info!("Ejected medium from drive '{}'.", &source_str);
                } else {
                    warn!(
                        "PowerShell eject command returned status {:?}.",
                        status.code()
                    );
                }
            }
            Err(error) => {
                warn!("Failed to execute PowerShell eject command: {}", error);
            }
        }
    } else {
        warn!(
            "Unable to eject, source '{}' is not a drive letter!",
            &source_str
        );
    };
}

pub fn eject_medium(source: &std::path::Path) {
    #[cfg(target_os = "linux")]
    {
        eject_medium_linux(source);
    }

    #[cfg(target_os = "windows")]
    {
        eject_medium_windows(source);
    }

    #[cfg(not(any(target_os = "linux", target_os = "windows")))]
    {
        warn!("Eject is not supported on this operating system.");
    }
}

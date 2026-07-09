/*
    eject a disc (OS-specific)
*/

#[allow(unused_imports)]
use log::{debug, error, info, warn};

use regex::Regex;

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

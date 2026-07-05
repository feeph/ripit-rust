/*
    extract data from a physical medium

    This code wraps 'makemkvcon backup' into a more convenient interface.
*/

mod os_utils;

use std::{fs, path::Path};

#[allow(unused_imports)]
use log::{debug, error, info, warn};

use makemkv::DriveRecord;

use crate::yaml_utils::generate_yaml;

fn find_drive_record(makemkvcon_bin: &Path, source: &str) -> Option<DriveRecord> {
    let (drive_records, _) = makemkv::drives(makemkvcon_bin);

    // find the drive matching the provided device name
    // - provides mapping from device name to disc id (required for backup)
    // - provides disc content type
    drive_records
        .into_iter()
        .find(|dr| dr.device_name == source)
}

pub enum UnshackleResult {
    ReadFailure,
    ScanFailure,
    ParseFailure,
    NoDrive,
    NoMedium,
    Success,
}

pub fn unshackle_disc(
    makemkvcon_bin: &Path,
    source: &Path,
    target: &Path,
    eject_when_done: bool,
    allow_overwrite: bool,
) -> UnshackleResult {
    let source_str = source.to_string_lossy();

    // the order of operations improves overall speed
    // 1. scan drives (fast, detect 'no drive' vs. 'no medium')
    // 2. identify medium (fast)
    // 3. extract from optical drive to disk (typically 10-20 minutes)
    // 4. scan the medium using the disk (faster than reading from drive)
    //
    // step 4 could be merged with step 2 but then we have to read the
    // medium two times from the optical drive; scanning from drive is
    // slower by a factor of 20 compared to scanning from disk (e.g. 6
    // seconds when reading from disk vs. 120 seconds reading from drive)

    // --------------------------------------------------------------------
    // step 1 - scan drives
    // --------------------------------------------------------------------

    debug!("step 2 - identify drive");
    // find the drive matching the provided device name or drive letter
    // - provides mapping from device name to disc id (required for backup)
    // - provides disc content type
    let drive = match find_drive_record(makemkvcon_bin, &source_str) {
        Some(dr) => {
            debug!(
                "Found optical drive: {} -> {}",
                dr.device_name, dr.disc_name
            );
            dr
        }
        None => {
            return UnshackleResult::NoDrive;
        }
    };

    // --------------------------------------------------------------------
    // step 2 - identify medium (volume label and block id)
    // --------------------------------------------------------------------

    debug!("step 1 - identify medium in drive '{}'.", source_str);
    let mut target_dump = std::path::PathBuf::from(target);
    let mut target_yaml = std::path::PathBuf::from(target);
    let result = os_utils::get_volume_id(&source_str);
    match result {
        Some(x) => {
            debug!(
                "Drive '{}' contains a medium with volume ID '{}'.",
                source_str, x
            );
            // append volume ID to target in order to create a unique
            // filesystem location for each disc (this is a precaution
            // since the volume name may be something stupid like
            // "DVDVolume", "LOGICAL_VOLUME_ID" or "UNDEFINED" and cause
            // filesystem conflicts if multiple discs share the same name)
            target_dump.push(&x);
            target_yaml.push(&x);
        }
        None => {
            return UnshackleResult::NoMedium;
        }
    }

    // ------------------------------------------------------------
    // step 3 - extract content
    // ------------------------------------------------------------

    debug!("step 3 - extract content");
    let disc_id = drive.index;
    if drive.content_type.has_dvd_files {
        // DVDs are extracted as images
        target_dump.add_extension("iso");
    }

    let result = makemkv::backup(makemkvcon_bin, disc_id, &target_dump, allow_overwrite);

    // --------------------------------------------------------------------
    // step 4 - scan extracted medium and save as YAML file
    // --------------------------------------------------------------------

    if result {
        debug!("step 4 - scan extracted content");
        let min_length = 0;
        target_yaml.add_extension("yaml");
        info!("target_yaml: {}", target_yaml.to_string_lossy());
        let scan_result =
            match makemkv::info(makemkvcon_bin, &target_dump.to_string_lossy(), min_length) {
                Some(x) => x,
                None => {
                    error!("Unable to scan '{}'!", source_str);
                    return UnshackleResult::ScanFailure;
                }
            };

        let yaml = generate_yaml(scan_result.parsed, true);
        match fs::write(&target_yaml, yaml) {
            Ok(_) => info!("Wrote scan output to '{}'.", target_yaml.display()),
            Err(err) => {
                error!(
                    "Failed to write output file '{}': {}",
                    target_yaml.display(),
                    err
                );
                return UnshackleResult::ParseFailure;
            }
        }

        if eject_when_done {
            info!("Ejecting medium from '{}'.", source_str);
            crate::eject::eject_medium(source);
        }
        UnshackleResult::Success
    } else {
        UnshackleResult::ReadFailure
    }
}

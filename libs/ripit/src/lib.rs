/*
    convenience wrapper for MakeMKV
*/

mod drives;
mod extract;
mod unshackle;
mod progress_update;
mod os_utils;
mod severities;

// ------------------------------------------------------------------------
// public interface
// ------------------------------------------------------------------------

pub use makemkv::ScanMode;

pub use drives::{OpticalDisc, OpticalDrive, DriveStatus, Dvd, HdDvd, BluRay, find_drives, find_matching_drive, find_matching_drives};
pub use extract::{ExtractError, ExtractEvent, ExtractResult, extract_from_drive, extract_from_image};
pub use progress_update::{ProgressUpdate, ProgressValue};
pub use unshackle::{UnshackleError, UnshackleEvent, unshackle_disc};

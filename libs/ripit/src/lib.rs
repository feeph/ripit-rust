/*
    convenience wrapper for MakeMKV
*/

mod drives;
mod extract;
mod os_utils;
mod progress_update;
mod severities;
mod unshackle;

// ------------------------------------------------------------------------
// public interface
// ------------------------------------------------------------------------

pub use makemkv::ScanMode;

pub use drives::{
    BluRay, DriveStatus, Dvd, HdDvd, OpticalDisc, OpticalDrive, find_drives, find_matching_drive,
    find_matching_drives,
};
pub use extract::{
    ExtractError, ExtractEvent, ExtractResult, extract_from_drive, extract_from_image,
};
pub use progress_update::{ProgressUpdate, ProgressValue};
pub use unshackle::{UnshackleError, UnshackleEvent, unshackle_disc};

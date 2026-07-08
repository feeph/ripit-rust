/*
    convenience wrapper for MakeMKV
*/

mod drives;
mod eject;
mod extract;
mod scan;
mod unshackle;

pub use eject::eject_medium;
pub use scan::{ScanResult, scan_disc};
pub use unshackle::{UnshackleError, unshackle_disc};

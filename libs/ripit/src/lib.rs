/*
*/

pub mod drives;
pub mod eject;
pub mod extract;
mod scan;
mod unshackle;

pub use scan::{ScanResult, scan_disc};
pub use unshackle::{UnshackleResult, unshackle_disc};

pub fn add(left: u64, right: u64) -> u64 {
    left + right
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn it_works() {
        let result = add(2, 2);
        assert_eq!(result, 4);
    }
}

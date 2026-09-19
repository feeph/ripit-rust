/*!
    provide functionality related to optical drives
*/

mod eject_disc;
mod optical_disc;
mod os_utils;

// standard library imports
use std::path::PathBuf;
use std::time::Duration;
use std::thread::sleep;

// third-party imports
// <none>

// crate-provided imports
use makemkv::{ContentType, DrvRecord};
use eject_disc::eject_disc;

// ------------------------------------------------------------------------
// public interface
// ------------------------------------------------------------------------

pub use optical_disc::{BluRay, Dvd, HdDvd, OpticalDisc};

#[derive(Clone, Debug, PartialEq, Eq, Hash, serde::Serialize)]
pub struct OpticalDrive {
    /// the device index (provided and used by makemkvcon)
    pub index: u8,

    /// the model, e.g. "BD-RE ASUS BW-16D1HT 3.10 KL1OBDB4635"
    pub model: String,

    /// the device name, e.g. "E:" or "/dev/sr0"
    pub device: PathBuf,

    /// it is unclear what this value means (provided by makemkvcon)
    pub is_enabled: u32,

    /// the inserted disc
    pub disc: Option<OpticalDisc>,
}

impl OpticalDrive {

    pub fn is_empty(&self) -> bool {
        self.disc.is_none()
    }

    pub fn has_disc(&self) -> bool {
        self.disc.is_some()
    }

    pub fn get_disc_name(&self) -> Option<&str> {
        match &self.disc {
            Some(disc) => Some(disc.get_name()),
            None => None,
        }
    }

    pub fn get_disc_type(&self) -> Option<&str> {
        match &self.disc {
            Some(disc) => Some(disc.get_type()),
            None => None,
        }
    }

    /// open the disc tray and/or eject the disc
    /// 
    /// - open the tray (if drive has one)
    /// - eject the disc (if disc is present)
    pub fn eject_disc(&self) {
        eject_disc(&self.device);
    }

}

// initialize from reference
impl From<&DrvRecord> for OpticalDrive {

    /// initialize from a DrvRecord reference
    fn from(dr: &DrvRecord) -> Self {
        let device = PathBuf::from(&dr.device_name);
        let model = dr.drive_name.to_owned();
        let index = dr.index;
        let is_enabled = dr.is_enabled;

        let sleep_time =  Duration::from_millis(500);
        let mut result = None;
        // try multiple times
        // (the first call may wake up an idle drive)
        for _ in 1..3 {
            match os_utils::get_volume_id(&device) {
                Some(uid) => {
                    result = Some(uid);
                    break;
                },
                None => {
                    sleep(sleep_time);
                }
            }
        }
        match result {
            Some(uid) => {
                let disc = parse_disc_type(&dr.volume_name, &uid, &dr.content_type);
                OpticalDrive {
                    device,
                    model, 
                    index,
                    is_enabled,
                    disc,
                }
            },
            None => {
                OpticalDrive {
                    device,
                    model, 
                    index,
                    is_enabled,
                    disc: None,
                }
            }
        }
    }

}

// initialize from owned object
impl From<DrvRecord> for OpticalDrive {

    /// initialize from a DrvRecord
    fn from(dr: DrvRecord) -> Self {
        OpticalDrive::from(&dr)
    }

}

// ------------------------------------------------------------------------
// helper functions
// ------------------------------------------------------------------------

fn parse_disc_type(name: &str, uid: &str, content_type: &ContentType) -> Option<OpticalDisc> {
    if content_type.has_dvd_files {
        Some(OpticalDisc::Dvd(Dvd{name: name.to_owned(), uid: uid.to_owned()}))
    } else if content_type.has_hddvd_files {
        Some(OpticalDisc::HdDvd(HdDvd{name: name.to_owned(), uid: uid.to_owned()}))
    } else if content_type.has_bluray_files {
        Some(OpticalDisc::BluRay(
            BluRay{
                name: name.to_owned(),
                uid: uid.to_owned(),
                has_aacs: content_type.has_aacs_files,
                has_bdsvm: content_type.has_bdsvm_files,
            }
        ))
    } else {
        None
    }
}

// ------------------------------------------------------------------------
// unit tests
// ------------------------------------------------------------------------

// FIXME the disc's volume id depends on the computer this test runs on -> make it system-independent for testing
#[cfg(test)]
mod tests {

    use super::*;

    #[test]
    fn test_init_from_drv_record_empty() {
        let data = b"DRV:0,0,999,0,\"DVD+R-DL PLDS DVD-RW DH16AFSH DL31 8SSDX0F17036L1CB5800MGJ\",\"\",\"/dev/sr0\"";
        let dr = makemkv::api::parse_drv_data(&data[4..]); // skip 'DRV:'
        // ----------------------------------------------------------------
        let computed = OpticalDrive::from(&dr);
        let expected = OpticalDrive {
            index: 0,
            model: "DVD+R-DL PLDS DVD-RW DH16AFSH DL31 8SSDX0F17036L1CB5800MGJ".to_string(),
            device: PathBuf::from("/dev/sr0"),
            is_enabled: 999,
            disc: None,
        };
        // ----------------------------------------------------------------
        assert_eq!(computed, expected);
    }

    #[test]
    fn test_init_from_drv_record_dvd() {
        let data = b"DRV:0,2,999,1,\"DVD+R-DL PLDS DVD-RW DH16AFSH DL31 8SSDX0F17036L1CB5800MGJ\",\"OTAKU_NO_VIDEO\",\"/dev/sr1\"";
        let dr = makemkv::api::parse_drv_data(&data[4..]); // skip 'DRV:'
        // ----------------------------------------------------------------
        let computed = OpticalDrive::from(&dr);
        let expected = OpticalDrive {
            index: 0,
            model: "DVD+R-DL PLDS DVD-RW DH16AFSH DL31 8SSDX0F17036L1CB5800MGJ".to_string(),
            device: PathBuf::from("/dev/sr1"),
            is_enabled: 999,
            disc: Some(
                OpticalDisc::Dvd(
                    Dvd {
                        name: "OTAKU_NO_VIDEO".to_string(),
                        uid: "3ed3dd1f5f5f5f4d".to_string(),
                    }
                )
            ),
        };
        // ----------------------------------------------------------------
        assert_eq!(computed, expected);
    }

    #[test]
    fn test_init_from_drv_record_bluray() {
        let data = b"DRV:2,2,999,12,\"BD-RE ASUS BW-16D1HT 3.10 KL1OBDB4635\",\"LOGICAL_VOLUME_ID\",\"/dev/sr0\"";
        let dr = makemkv::api::parse_drv_data(&data[4..]); // skip 'DRV:'
        // ----------------------------------------------------------------
        let computed = OpticalDrive::from(&dr);
        let expected = OpticalDrive {
            index: 2,
            model: "BD-RE ASUS BW-16D1HT 3.10 KL1OBDB4635".to_string(),
            device: PathBuf::from("/dev/sr0"),
            is_enabled: 999,
            disc: Some(
                OpticalDisc::BluRay(
                    BluRay {
                        name: "LOGICAL_VOLUME_ID".to_string(),
                        uid: "091716445f464557".to_string(),
                        has_aacs: true,
                        has_bdsvm: false,
                    }
                )
            ),
        };
        // ----------------------------------------------------------------
        assert_eq!(computed, expected);
    }

    /// basically the same as test_init_from_drv_record_empty() except that
    /// this test confirms we can initialize from an owned object instead
    /// of a borrowed reference.
    #[test]
    fn test_init_from_owned_object() {
        let data = b"DRV:0,0,999,0,\"DVD+R-DL PLDS DVD-RW DH16AFSH DL31 8SSDX0F17036L1CB5800MGJ\",\"\",\"/dev/sr0\"";
        let dr = makemkv::api::parse_drv_data(&data[4..]); // skip 'DRV:'
        // ----------------------------------------------------------------
        let computed = OpticalDrive::from(dr.clone());
        let expected = OpticalDrive::from(&dr);
        // ----------------------------------------------------------------
        assert_eq!(computed, expected);
    }

}

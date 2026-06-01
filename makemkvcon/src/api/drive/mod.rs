/*
    drive record (DRV)
*/
// TODO figure out how disc-changers are mapped

mod content_type;

use content_type::{parse_content_type_value};

use csv;
use num::FromPrimitive;

pub use crate::apdefs_h::{DriveStatus};
pub use content_type::{ContentType};

#[derive(Clone, Debug, PartialEq, serde::Serialize)]
pub struct DriveRecord {
    pub index: u8,
    // 'visible' - set to 1 if drive is present
    pub drive_status_num: u32,  // numerical representation
    pub drive_status: Option<DriveStatus>, // parsed value
    // 'enabled' - set to 1 if drive is accessible
    pub is_enabled: u32,
    // 'flags' - media flags, see AP_DskFsFlagXXX in apdefs.h
    pub content_type_num: u8,  // numerical representation
    pub content_type: ContentType, // parsed value
    pub drive_name: String,
    pub disc_name: String,
    pub device_name: String,
}

pub fn parse_drive_record_data(data: &[u8]) -> DriveRecord {
    // A separate CSV reader instance is created for every parsed line.
    // This is probably okay since the limiting factor here is "reading
    // from a physical medium", not "lines parsed per millisecond".
    // TODO validate that the CSV-reader is sufficiently performant
    let mut rdr = csv::ReaderBuilder::new()
        .has_headers(false)
        .from_reader(data);
    let record = rdr.records().next().unwrap().unwrap();
    let fields: Vec<String> = record.iter().map(|s| s.to_string()).collect();

    let index_u8 = fields[0].parse::<u8>().unwrap();
    let drive_status_u32 = fields[1].parse::<u32>().unwrap();
    // according to usage.txt 'is_enabled' is supposed to be 0 or 1 but
    // it's typically set to 999
    let is_enabled_u32 = fields[2].parse::<u32>().unwrap();
    let content_type_id_u8 = fields[3].parse::<u8>().unwrap();
    let drive_name_str = fields[4].clone();
    let disc_name_str = fields[5].clone();
    let device_name_str = fields[6].clone();

    DriveRecord {
        index: index_u8,
        drive_status_num: drive_status_u32,
        drive_status: DriveStatus::from_u32(drive_status_u32),
        is_enabled: is_enabled_u32,
        content_type_num: content_type_id_u8,
        content_type: parse_content_type_value(content_type_id_u8),
        drive_name: drive_name_str,
        disc_name: disc_name_str,
        device_name: device_name_str,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    // drive closed but no disc present
    // DRV:0,0,999,0,"DVD+R-DL PLDS DVD-RW DH16AFSH DL31 8SSDX0F17036L1CB5800MGJ","","/dev/sr1"
    #[test]
    fn parse_empty_closed() {
        let data = b"0,0,999,0,\"DVD+R-DL PLDS DVD-RW DH16AFSH DL31 8SSDX0F17036L1CB5800MGJ\",\"\",\"/dev/sr1\"";
        // ----------------------------------------------------------------
        let computed = parse_drive_record_data(data);
        let expected = DriveRecord {
            index: 0,
            drive_status_num: 0,
            drive_status: Some(DriveStatus::EmptyClosed),
            is_enabled: 999,
            content_type_num: 0,
            content_type: ContentType {
                has_dvd_files: false,
                has_hddvd_files: false,
                has_bluray_files: false,
                has_aacs_files: false,
                has_bdsvm_files: false,
            },
            drive_name: "DVD+R-DL PLDS DVD-RW DH16AFSH DL31 8SSDX0F17036L1CB5800MGJ".to_string(),
            disc_name: "".to_string(),
            device_name: "/dev/sr1".to_string(),
        };
        // ----------------------------------------------------------------
        assert_eq!(computed, expected);
    }

    // drive open / no disc present / no disc detected
    // DRV:0,1,999,0,"BD-RE ASUS BW-16D1HT 3.10 KL1OBDB4635","","/dev/sr0"
    #[test]
    fn parse_empty_open() {
        let data = b"0,1,999,0,\"BD-RE ASUS BW-16D1HT 3.10 KL1OBDB4635\",\"\",\"/dev/sr0\"";
        // ----------------------------------------------------------------
        let computed = parse_drive_record_data(data);
        let expected = DriveRecord {
            index: 0,
            drive_status_num: 1,
            drive_status: Some(DriveStatus::EmptyOpen),
            is_enabled: 999,
            content_type_num: 0,
            content_type: ContentType {
                has_dvd_files: false,
                has_hddvd_files: false,
                has_bluray_files: false,
                has_aacs_files: false,
                has_bdsvm_files: false,
            },
            drive_name: "BD-RE ASUS BW-16D1HT 3.10 KL1OBDB4635".to_string(),
            disc_name: "".to_string(),
            device_name: "/dev/sr0".to_string(),
        };
        // ----------------------------------------------------------------
        assert_eq!(computed, expected);
    }
    
    // DVD detected (Linux, using device name)
    // DRV:0,2,999,1,"BD-RE ASUS BW-16D1HT 3.10 KL1OBDB4635","Nekomonogatari White","/dev/sr0"
    #[test]
    fn parse_dvd_present_linux() {
        let data = b"0,2,999,1,\"BD-RE ASUS BW-16D1HT 3.10 KL1OBDB4635\",\"Nekomonogatari White\",\"/dev/sr0\"";
        // ----------------------------------------------------------------
        let computed = parse_drive_record_data(data);
        let expected = DriveRecord {
            index: 0,
            drive_status_num: 2,
            drive_status: Some(DriveStatus::DiscInserted),
            is_enabled: 999,
            content_type_num: 1,
            content_type: ContentType {
                has_dvd_files: true,
                has_hddvd_files: false,
                has_bluray_files: false,
                has_aacs_files: false,
                has_bdsvm_files: false,
            },
            drive_name: "BD-RE ASUS BW-16D1HT 3.10 KL1OBDB4635".to_string(),
            disc_name: "Nekomonogatari White".to_string(),
            device_name: "/dev/sr0".to_string(),
        };
        // ----------------------------------------------------------------
        assert_eq!(computed, expected);
    }

    // DVD detected (Windows, using drive letter)
    // DRV:0,2,999,1,"BD-RE ASUS SBW-06D5H-U E101 AFDL222859WL","COWBOY_BEBOP_V4","E:"
    #[test]
    fn parse_dvd_present_windows() {
        let data = b"0,2,999,1,\"BD-RE ASUS SBW-06D5H-U E101 AFDL222859WL\",\"COWBOY_BEBOP_V4\",\"E:\"";
        // ----------------------------------------------------------------
        let computed = parse_drive_record_data(data);
        let expected = DriveRecord {
            index: 0,
            drive_status_num: 2,
            drive_status: Some(DriveStatus::DiscInserted),
            is_enabled: 999,
            content_type_num: 1,
            content_type: ContentType {
                has_dvd_files: true,
                has_hddvd_files: false,
                has_bluray_files: false,
                has_aacs_files: false,
                has_bdsvm_files: false,
            },
            drive_name: "BD-RE ASUS SBW-06D5H-U E101 AFDL222859WL".to_string(),
            disc_name: "COWBOY_BEBOP_V4".to_string(),
            device_name: "E:".to_string(),
        };
        // ----------------------------------------------------------------
        assert_eq!(computed, expected);
    }

    // BluRay detected
    // DRV:0,2,999,12,"BD-RE ASUS BW-16D1HT 3.10 KL1OBDB4635","NIKITA","/dev/sr0"
    #[test]
    fn parse_bluray_present() {
        let data = b"0,2,999,12,\"BD-RE ASUS BW-16D1HT 3.10 KL1OBDB4635\",\"NIKITA\",\"/dev/sr0\"";
        // ----------------------------------------------------------------
        let computed = parse_drive_record_data(data);
        let expected = DriveRecord {
            index: 0,
            drive_status_num: 2,
            drive_status: Some(DriveStatus::DiscInserted),
            is_enabled: 999,
            content_type_num: 12,
            content_type: ContentType {
                has_dvd_files: false,
                has_hddvd_files: false,
                has_bluray_files: true,
                has_aacs_files: true,
                has_bdsvm_files: false,
            },
            drive_name: "BD-RE ASUS BW-16D1HT 3.10 KL1OBDB4635".to_string(),
            disc_name: "NIKITA".to_string(),
            device_name: "/dev/sr0".to_string(),
        };
        // ----------------------------------------------------------------
        assert_eq!(computed, expected);
    }

    // drive not present
    // DRV:15,256,999,0,"","",""
    #[test]
    fn parse_drive_not_present() {
        let data = b"15,256,999,0,\"\",\"\",\"\"";
        // ----------------------------------------------------------------
        let computed = parse_drive_record_data(data);
        let expected = DriveRecord {
            index: 15,
            drive_status_num: 256,
            drive_status: Some(DriveStatus::NoDrive),
            is_enabled: 999,
            content_type_num: 0,
            content_type: ContentType {
                has_dvd_files: false,
                has_hddvd_files: false,
                has_bluray_files: false,
                has_aacs_files: false,
                has_bdsvm_files: false,
            },
            drive_name: "".to_string(),
            disc_name: "".to_string(),
            device_name: "".to_string(),
        };
        // ----------------------------------------------------------------
        assert_eq!(computed, expected);
    }

}

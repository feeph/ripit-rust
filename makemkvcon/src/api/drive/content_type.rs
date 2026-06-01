/*
    MakeMKV encodes content type as an integer, each bit represents
    specific files found on the media. A combination of multiple flags
    is possible, e.g. BluRay discs typically have BluRay and AACS files.

    The bitmask values must match the ones defined in
    in `makemkv-oss-1.18.3/makemkvgui/inc/lgpl/apdefs.h`.

    bitmask     identifier in apdefs.h
    -------------------------------------------
    0b000_0001  AP_DskFsFlagDvdFilesPresent
    0b000_0010  AP_DskFsFlagHdvdFilesPresent
    0b000_0100  AP_DskFsFlagBlurayFilesPresent
    0b000_1000  AP_DskFsFlagAacsFilesPresent
    0b001_0000  AP_DskFsFlagBdsvmFilesPresent
*/

#[derive(Clone, Debug, PartialEq, serde::Serialize)]
pub struct ContentType {
    pub has_dvd_files: bool,
    pub has_hddvd_files: bool,
    pub has_bluray_files: bool,
    pub has_aacs_files: bool,
    pub has_bdsvm_files: bool,
}

pub fn parse_content_type_value(value: u8) -> ContentType {
    let has_dvd_files = value & 0b000_0001 != 0;
    let has_hddvd_files = value & 0b000_0010 != 0;
    let has_bluray_files = value & 0b000_0100 != 0;
    let has_aacs_files = value & 0b000_1000 != 0;
    let has_bdsvm_files = value & 0b001_0000 != 0;

    ContentType {
        has_dvd_files,
        has_hddvd_files,
        has_bluray_files,
        has_aacs_files,
        has_bdsvm_files,
    }    
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_no_disc() {
        let data: u8 = 0;
        // ----------------------------------------------------------------
        let computed = parse_content_type_value(data);
        let expected = ContentType {
            has_dvd_files: false,
            has_hddvd_files: false,
            has_bluray_files: false,
            has_aacs_files: false,
            has_bdsvm_files: false,
        };
        // ----------------------------------------------------------------
        assert_eq!(computed, expected);
    }

    #[test]
    fn parse_dvd() {
        let data: u8 = 1;
        // ----------------------------------------------------------------
        let computed = parse_content_type_value(data);
        let expected = ContentType {
            has_dvd_files: true,
            has_hddvd_files: false,
            has_bluray_files: false,
            has_aacs_files: false,
            has_bdsvm_files: false,
        };
        // ----------------------------------------------------------------
        assert_eq!(computed, expected);
    }

    #[test]
    fn parse_hd_dvd() {
        let data: u8 = 2;
        // ----------------------------------------------------------------
        let computed = parse_content_type_value(data);
        let expected = ContentType {
            has_dvd_files: false,
            has_hddvd_files: true,
            has_bluray_files: false,
            has_aacs_files: false,
            has_bdsvm_files: false,
        };
        // ----------------------------------------------------------------
        assert_eq!(computed, expected);
    }

    #[test]
    fn parse_bluray() {
        let data: u8 = 12; // BluRay (4) + AACS (12)
        // ----------------------------------------------------------------
        let computed = parse_content_type_value(data);
        let expected = ContentType {
            has_dvd_files: false,
            has_hddvd_files: false,
            has_bluray_files: true,
            has_aacs_files: true,
            has_bdsvm_files: false,
        };
        // ----------------------------------------------------------------
        assert_eq!(computed, expected);
    }

}

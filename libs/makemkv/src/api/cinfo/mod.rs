/*
    content info record (CINFO)
*/

use crate::api::info::InfoRecord;

#[allow(unused_imports)]
use log::{debug, error, info, warn};

use crate::api::line_parser::parse_line;

pub fn parse_cinfo_data(data: &[u8]) -> InfoRecord {
    let fields = parse_line(data);

    let attr = fields[0].parse::<u32>().unwrap();
    let code = fields[1].parse::<u32>().unwrap();
    let value = fields[2].clone();

    InfoRecord::new(attr, code, &value)
}

#[cfg(test)]
mod tests {
    use super::*;

    // CINFO:1,6206,"DVD disc"
    #[test]
    fn parse_cinfo_type() {
        let data = b"1,6206,\"DVD disc\"";
        // ----------------------------------------------------------------
        let computed = parse_cinfo_data(data);
        let expected = InfoRecord::new(1, 6206, "DVD disc");
        // ----------------------------------------------------------------
        assert_eq!(computed, expected);
    }

    // CINFO:2,0,"Disc 1"
    #[test]
    fn parse_cinfo_name() {
        let data = b"2,0,\"Disc 1\"";
        // ----------------------------------------------------------------
        let computed = parse_cinfo_data(data);
        let expected = InfoRecord::new(2, 0, "Disc 1");
        // ----------------------------------------------------------------
        assert_eq!(computed, expected);
    }

    // CINFO:31,6119,"<b>Source information</b><br>"
    #[test]
    fn parse_cinfo_panel_title() {
        let data = b"31,6119,\"<b>Source information</b><br>\"";
        // ----------------------------------------------------------------
        let computed = parse_cinfo_data(data);
        let expected = InfoRecord::new(31, 6119, "<b>Source information</b><br>");
        // ----------------------------------------------------------------
        assert_eq!(computed, expected);
    }
}

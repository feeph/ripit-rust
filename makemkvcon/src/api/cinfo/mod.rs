/*
    content info record (CINFO)
*/

use crate::api::info::InfoRecord;

#[allow(unused_imports)]
use log::{debug, error, info, warn};

use csv;

pub fn parse_content_info_data(data: &[u8]) -> InfoRecord {
    // A separate CSV reader instance is created for every parsed line.
    // This is probably okay since the limiting factor here is "reading
    // from a physical medium", not "lines parsed per millisecond".
    // TODO validate that the CSV-reader is sufficiently performant
    let mut rdr = csv::ReaderBuilder::new()
        .has_headers(false)
        .from_reader(data);
    let record = rdr.records().next().unwrap().unwrap();
    let fields: Vec<String> = record.iter().map(|s| s.to_string()).collect();

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
        let computed = parse_content_info_data(data);
        let expected = InfoRecord::new(1, 6206, "DVD disc");
        // ----------------------------------------------------------------
        assert_eq!(computed, expected);
    }

    // CINFO:2,0,"Disc 1"
    #[test]
    fn parse_cinfo_name() {
        let data = b"2,0,\"Disc 1\"";
        // ----------------------------------------------------------------
        let computed = parse_content_info_data(data);
        let expected = InfoRecord::new(2, 0, "Disc 1");
        // ----------------------------------------------------------------
        assert_eq!(computed, expected);
    }

    // CINFO:31,6119,"<b>Source information</b><br>"
    #[test]
    fn parse_cinfo_panel_title() {
        let data = b"31,6119,\"<b>Source information</b><br>\"";
        // ----------------------------------------------------------------
        let computed = parse_content_info_data(data);
        let expected = InfoRecord::new(31, 6119, "<b>Source information</b><br>");
        // ----------------------------------------------------------------
        assert_eq!(computed, expected);
    }

}

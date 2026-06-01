/*
    title info record (TINFO)
*/

use crate::api::info::InfoRecord;

#[allow(unused_imports)]
use log::{debug, error, info, warn};

use csv;

pub fn parse_title_info_data(data: &[u8]) -> (usize, InfoRecord) {
    // A separate CSV reader instance is created for every parsed line.
    // This is probably okay since the limiting factor here is "reading
    // from a physical medium", not "lines parsed per millisecond".
    // TODO validate that the CSV-reader is sufficiently performant
    let mut rdr = csv::ReaderBuilder::new()
        .has_headers(false)
        .from_reader(data);
    let record = rdr.records().next().unwrap().unwrap();
    let fields: Vec<String> = record.iter().map(|s| s.to_string()).collect();

    let tid = fields[0].parse::<usize>().unwrap();

    let attr = fields[1].parse::<u32>().unwrap();
    let code = fields[2].parse::<u32>().unwrap();
    let value = fields[3].clone();

    let info = InfoRecord::new(attr, code, &value);

    (tid, info)
}

#[cfg(test)]
mod tests {
    use super::*;

    // TINFO:2,8,0,"1"
    #[test]
    fn parse_tinfo_tid() {
        let data = b"2,8,0,\"1\"";
        // ----------------------------------------------------------------
        let computed = parse_title_info_data(data).0;
        let expected = 2;
        // ----------------------------------------------------------------
        assert_eq!(computed, expected);
    }

    // TINFO:0,31,6120,"<b>Title information</b><br>"
    #[test]
    fn parse_tinfo_record() {
        let data = b"0,31,6120,\"<b>Title information</b><br>\"";
        // ----------------------------------------------------------------
        let computed = parse_title_info_data(data).1;
        let expected = InfoRecord::new(31, 6120, "<b>Title information</b><br>");
        // ----------------------------------------------------------------
        assert_eq!(computed, expected);
    }

}

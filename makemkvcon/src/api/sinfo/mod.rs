/*
    stream info record (SINFO)
*/

use crate::api::info::InfoRecord;

#[allow(unused_imports)]
use log::{debug, error, info, warn};

use csv;

pub fn parse_stream_info_data(data: &[u8]) -> (usize, usize, InfoRecord) {
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
    let sid = fields[1].parse::<usize>().unwrap();

    let attr = fields[2].parse::<u32>().unwrap();
    let code = fields[3].parse::<u32>().unwrap();
    let value = fields[4].clone();

    let info = InfoRecord::new(attr, code, &value);

    (tid, sid, info)
}

#[cfg(test)]
mod tests {
    use super::*;

    // SINFO:2,0,1,6201,"Video"
    #[test]
    fn parse_sinfo_tid() {
        let data = b"2,0,1,6201,\"Video\"";
        // ----------------------------------------------------------------
        let computed = parse_stream_info_data(data).0;
        let expected = 2;
        // ----------------------------------------------------------------
        assert_eq!(computed, expected);
    }

    // SINFO:2,0,1,6201,"Video"
    #[test]
    fn parse_sinfo_sid() {
        let data = b"2,0,1,6201,\"Video\"";
        // ----------------------------------------------------------------
        let computed = parse_stream_info_data(data).1;
        let expected = 0;
        // ----------------------------------------------------------------
        assert_eq!(computed, expected);
    }

    // SINFO:2,0,1,6201,"Video"
    #[test]
    fn parse_sinfo_info() {
        let data = b"2,0,1,6201,\"Video\"";
        // ----------------------------------------------------------------
        let computed = parse_stream_info_data(data).2;
        let expected = InfoRecord::new(1, 6201, "Video");
        // ----------------------------------------------------------------
        assert_eq!(computed, expected);
    }
}

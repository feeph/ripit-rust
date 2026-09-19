/*
    stream info record (SINFO)
*/

use crate::api::info::InfoRecord;

#[allow(unused_imports)]
use log::{debug, error, info, warn};

use crate::api::line_parser::parse_line;

pub fn parse_sinfo_data(data: &[u8]) -> (usize, usize, InfoRecord) {
    let fields = parse_line(data);

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
        let computed = parse_sinfo_data(data).0;
        let expected = 2;
        // ----------------------------------------------------------------
        assert_eq!(computed, expected);
    }

    // SINFO:2,0,1,6201,"Video"
    #[test]
    fn parse_sinfo_sid() {
        let data = b"2,0,1,6201,\"Video\"";
        // ----------------------------------------------------------------
        let computed = parse_sinfo_data(data).1;
        let expected = 0;
        // ----------------------------------------------------------------
        assert_eq!(computed, expected);
    }

    // SINFO:2,0,1,6201,"Video"
    #[test]
    fn parse_sinfo_info() {
        let data = b"2,0,1,6201,\"Video\"";
        // ----------------------------------------------------------------
        let computed = parse_sinfo_data(data).2;
        let expected = InfoRecord::new(1, 6201, "Video");
        // ----------------------------------------------------------------
        assert_eq!(computed, expected);
    }
}

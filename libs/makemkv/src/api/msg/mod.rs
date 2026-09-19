/*
    message record (MSG)
*/

#[allow(unused_imports)]
use log::{debug, error, info, warn};

use crate::api::line_parser::parse_line;

#[derive(Clone, Debug, PartialEq)]
pub struct MsgRecord {
    pub code: u32,
    pub flags: u32,
    pub count: u32,
    pub message: String,
    pub format: String,
    pub params: Vec<String>,
}

pub fn parse_msg_data(data: &[u8]) -> MsgRecord {
    let fields = parse_line(data);

    let code_u32 = fields[0].parse::<u32>().unwrap();
    let flags_u32 = fields[1].parse::<u32>().unwrap();
    let count_u32 = fields[2].parse::<u32>().unwrap();

    // extract remaining values (if any) as a vector of strings
    let params = if fields.len() > 5 {
        fields[5..].to_vec()
    } else {
        Vec::new()
    };

    MsgRecord {
        code: code_u32,
        flags: flags_u32,
        count: count_u32,
        message: fields[3].clone(),
        format: fields[4].clone(),
        params,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    // MSG:3007,0,0,"Using direct disc access mode","Using direct disc access mode"
    #[test]
    fn parse_msg_without_value() {
        let data = b"3007,0,0,\"Using direct disc access mode\",\"Using direct disc access mode\"";
        // ----------------------------------------------------------------
        let computed = parse_msg_data(data);
        let expected = MsgRecord {
            code: 3007,
            flags: 0,
            count: 0,
            message: "Using direct disc access mode".to_string(),
            format: "Using direct disc access mode".to_string(),
            params: Vec::from([]),
        };
        // ----------------------------------------------------------------
        assert_eq!(computed, expected);
    }

    // MSG:1005,0,1,"MakeMKV v1.17.9 linux(x64-release) started","%1 started","MakeMKV v1.17.9 linux(x64-release)"
    #[test]
    fn parse_msg_with_single_value() {
        let data = b"1005,0,1,\"MakeMKV v1.17.9 linux(x64-release) started\",\"%1 started\",\"MakeMKV v1.17.9 linux(x64-release)\"";
        // ----------------------------------------------------------------
        let computed = parse_msg_data(data);
        let expected = MsgRecord {
            code: 1005,
            flags: 0,
            count: 1,
            message: "MakeMKV v1.17.9 linux(x64-release) started".to_string(),
            format: "%1 started".to_string(),
            params: Vec::from(["MakeMKV v1.17.9 linux(x64-release)".to_string()]),
        };
        // ----------------------------------------------------------------
        assert_eq!(computed, expected);
    }

    // MSG:3028,0,3,"Title #1 was added (1 cell(s), 0:23:39)","Title #%1 was added (%2 cell(s), %3)","1","1","0:23:39"
    #[test]
    fn parse_msg_with_multiple_values() {
        let data = b"3028,0,3,\"Title #1 was added (1 cell(s), 0:23:39)\",\"Title #%1 was added (%2 cell(s), %3)\",\"1\",\"1\",\"0:23:39\"";
        // ----------------------------------------------------------------
        let computed = parse_msg_data(data);
        let expected = MsgRecord {
            code: 3028,
            flags: 0,
            count: 3,
            message: "Title #1 was added (1 cell(s), 0:23:39)".to_string(),
            format: "Title #%1 was added (%2 cell(s), %3)".to_string(),
            params: Vec::from(["1".to_string(), "1".to_string(), "0:23:39".to_string()]),
        };
        // ----------------------------------------------------------------
        assert_eq!(computed, expected);
    }
}

/*
    data objects
*/

use std::collections::HashMap;

use crate::api::DriveRecord;

#[derive(Clone, Debug, PartialEq, serde::Serialize)]
pub struct InfoRecordOut {
    pub code: String,
    pub value: String,
}

impl InfoRecordOut {
    pub fn from(code: &str, value: &str) -> Self {
        InfoRecordOut {
            code: code.to_string(),
            value: value.to_string(),
        }
    }
}

#[derive(Clone, Debug, PartialEq, serde::Serialize)]
pub struct StreamRecord {
    pub attributes: HashMap<String, InfoRecordOut>,
}

impl StreamRecord {
    pub fn new() -> Self {
        Self {
            attributes: HashMap::new(),
        }
    }
}

impl Default for StreamRecord {
    fn default() -> Self {
        StreamRecord::new()
    }
}

#[derive(Clone, Debug, PartialEq, serde::Serialize)]
pub struct MakeMkvConfig {
    pub min_length: usize,
}

#[derive(Clone, Debug, PartialEq, serde::Serialize)]
pub struct MakeMkvRecord {
    pub version: String,
    pub config: MakeMkvConfig,
}

#[derive(Clone, Debug, PartialEq, serde::Serialize)]
pub struct TitleRecord {
    pub attributes: HashMap<String, InfoRecordOut>,
    pub streams: HashMap<usize, StreamRecord>,
}

impl TitleRecord {
    pub fn new() -> Self {
        Self {
            attributes: HashMap::new(),
            streams: HashMap::new(),
        }
    }
}

impl Default for TitleRecord {
    fn default() -> Self {
        TitleRecord::new()
    }
}

#[derive(Clone, Debug, PartialEq, serde::Serialize)]
pub struct ContentRecord {
    pub info: HashMap<String, InfoRecordOut>,
    pub titles: HashMap<usize, TitleRecord>,
}

#[derive(Debug, PartialEq, serde::Serialize)]
pub struct ParsedOutput {
    pub drives: Vec<DriveRecord>,
    pub content: ContentRecord,
    pub makemkv: MakeMkvRecord,
    // ----------------------------------------
    pub issues: usize,
    pub errors: usize,
}

mod tests {

    // cargo complains that 'use super::*' is unused but it's needed
    #[allow(unused_imports)]
    use super::*;

    #[test]
    fn test_construct_new_stream_record() {
        // ----------------------------------------------------------------
        let computed = StreamRecord::new();
        let expected = StreamRecord {
            attributes: HashMap::new(),
        };
        // ----------------------------------------------------------------
        assert_eq!(computed, expected);
    }

    #[test]
    fn test_construct_default_stream_record() {
        // ----------------------------------------------------------------
        let computed = StreamRecord::default();
        let expected = StreamRecord {
            attributes: HashMap::new(),
        };
        // ----------------------------------------------------------------
        assert_eq!(computed, expected);
    }

    #[test]
    fn test_construct_new_title_record() {
        // ----------------------------------------------------------------
        let computed = TitleRecord::new();
        let expected = TitleRecord {
            attributes: HashMap::new(),
            streams: HashMap::new(),
        };
        // ----------------------------------------------------------------
        assert_eq!(computed, expected);
    }

    #[test]
    fn test_construct_default_title_record() {
        // ----------------------------------------------------------------
        let computed = TitleRecord::default();
        let expected = TitleRecord {
            attributes: HashMap::new(),
            streams: HashMap::new(),
        };
        // ----------------------------------------------------------------
        assert_eq!(computed, expected);
    }
}

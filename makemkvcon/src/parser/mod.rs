/*
    output parser for makemkvcon
*/

use std::collections::HashMap;

use crate::api::{
    DriveRecord, InfoRecord, MessageRecord, parse_content_info_data, parse_drive_record_data,
    parse_msg_data, parse_stream_info_data, parse_title_count_data, parse_title_info_data,
};

use async_stream::stream;
use futures::Stream;
use tokio::io::{AsyncBufReadExt, BufReader};
use tokio::process::Command;

#[allow(unused_imports)]
use log::{debug, error, info, warn};

#[allow(clippy::upper_case_acronyms)]
#[derive(Debug, PartialEq)]
enum ParsedOutputLine {
    DRV(DriveRecord),
    MSG(MessageRecord),
    CINFO(InfoRecord),
    TINFO((usize, InfoRecord)),
    SINFO((usize, usize, InfoRecord)),
    TCOUNT(usize),
}

fn parse_output_line(line: &[u8]) -> ParsedOutputLine {
    // split input at first colon
    // "<id>:<data>" -> "<id>" and ":<data>"
    let idx = line.iter().position(|x| x == &b':').unwrap();
    let (id, data) = line.split_at(idx);

    match id {
        // skip the leading ':' in data
        b"MSG" => ParsedOutputLine::MSG(parse_msg_data(&data[1..])),
        b"DRV" => ParsedOutputLine::DRV(parse_drive_record_data(&data[1..])),
        b"TCOUNT" => ParsedOutputLine::TCOUNT(parse_title_count_data(&data[1..])),
        b"TINFO" => ParsedOutputLine::TINFO(parse_title_info_data(&data[1..])),
        b"CINFO" => ParsedOutputLine::CINFO(parse_content_info_data(&data[1..])),
        b"SINFO" => ParsedOutputLine::SINFO(parse_stream_info_data(&data[1..])),
        _ => panic!("Found unsupported ID {}!", std::str::from_utf8(id).unwrap()),
    }
}

#[allow(clippy::upper_case_acronyms)]
#[derive(Debug, PartialEq)]
pub enum DiscContent {
    VERSION(String),
    CINFO(InfoRecord),
    DRV(DriveRecord),
    SINFO((usize, usize, InfoRecord)),
    TCOUNT(usize),
    TINFO((usize, InfoRecord)),
}

// line-based - read the output while makemkv is running
// Windows: "cmd /c makemkvcon64.exe --robot info dvd.iso"
pub async fn parse_command_output(command: &str) -> impl Stream<Item = DiscContent> + use<'_> {
    stream! {
        let mut child = Command::new("cmd")
            .arg("/c")
            .arg(command)
            .stdout(std::process::Stdio::piped())
            .spawn()
            .unwrap();

        let stdout = child.stdout.take().unwrap();
        let reader = BufReader::new(stdout);
        let mut lines = reader.lines();

        let mut parse_started = false;
        let mut issues = 0;
        while let Ok(Some(line)) = lines.next_line().await {
            let parsed = parse_output_line(line.as_bytes());
            // match conditions sorted in order of occurrence
            match parsed {
                // 'v' = 'value'
                ParsedOutputLine::MSG(v) => {
                    if parse_started {
                        issues += 1;
                    }
                    log::info!("[makemkvcon] {}", v.message)
                },
                ParsedOutputLine::DRV(v) => yield DiscContent::DRV(v),
                ParsedOutputLine::TCOUNT(v) => {
                    parse_started = true;
                    yield DiscContent::TCOUNT(v)
                },
                ParsedOutputLine::CINFO(v) => yield DiscContent::CINFO(v),
                ParsedOutputLine::TINFO(v) => yield DiscContent::TINFO(v),
                ParsedOutputLine::SINFO(v) => yield DiscContent::SINFO(v),
            }
        }
        if issues > 0 {
            log::warn!("Detected {} potential issues during parsing! Please validate.", issues)
        }
    }
}

// data-based - output was already generated, we just need to parse it
pub fn parse_canned_output(output: &Vec<&[u8]>) -> (Vec<DiscContent>, u32) {
    let mut parse_started = false;
    let mut issues = 0u32;

    let mut records = Vec::new();
    for line in output.iter() {
        let parsed = parse_output_line(line);
        match parsed {
            // special handling for MSG and TCOUNT
            ParsedOutputLine::MSG(m) => {
                if parse_started {
                    issues += 1;
                }
                log::info!("[makemkvcon] {}", m.message)
            }
            ParsedOutputLine::TCOUNT(t) => {
                parse_started = true;
                records.push(DiscContent::TCOUNT(t))
            }
            // everything else gets passed through
            ParsedOutputLine::DRV(x) => records.push(DiscContent::DRV(x)),
            ParsedOutputLine::CINFO(x) => records.push(DiscContent::CINFO(x)),
            ParsedOutputLine::SINFO(x) => records.push(DiscContent::SINFO(x)),
            ParsedOutputLine::TINFO(x) => records.push(DiscContent::TINFO(x)),
        }
    }
    if issues > 0 {
        log::warn!(
            "Detected {} potential issues during parsing! Please validate.",
            issues
        )
    }

    (records, issues)
}

// ------------------------------------------------------------------------

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

pub fn convert_info_record(info: InfoRecord) -> (String, InfoRecordOut) {
    let attr_str: String = match info.attr_val {
        Some(x) => x.to_string(),
        None => format!("Unknown ({})", info.attr_num),
    };
    let code_str: String = match info.code_val {
        Some(x) => x.to_string(),
        None => format!("Unknown ({})", info.code_num),
    };

    (
        attr_str,
        InfoRecordOut {
            code: code_str,
            value: info.value,
        },
    )
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

// ------------------------------------------------------------------------

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

// ------------------------------------------------------------------------

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

pub enum SeverityLevel {
    Debug,
    Error,
    Info,
    Warning,
}

pub fn process_output<R: std::io::BufRead>(
    reader: R,
    min_length: usize,
    severity_map: &HashMap<u32, SeverityLevel>,
) -> ParsedOutput {
    // the output returned by makemkvcon is context-sensitive and follows
    // this structure:
    // --------------------------------------------------------------------
    // DRV
    // TCOUNT
    // CINFO
    // TINFO - title #0
    // SINFO - streams for title #0
    // TINFO - title #1
    // SINFO - streams for title #1
    // --------------------------------------------------------------------

    let mut parsed_output = ParsedOutput {
        drives: Vec::new(),
        makemkv: MakeMkvRecord {
            version: "<n/a>".to_string(),
            config: MakeMkvConfig { min_length },
        },
        content: ContentRecord {
            info: HashMap::new(),
            titles: HashMap::new(),
        },
        issues: 0,
        errors: 0,
    };
    let mut tc_want: usize = 0;
    for line in reader.lines() {
        match line {
            Ok(line) => {
                debug!("{}", line);
                let parsed = parse_output_line(line.as_bytes());
                match parsed {
                    ParsedOutputLine::MSG(m) => {
                        match severity_map.get(&m.code) {
                            Some(SeverityLevel::Debug) => {
                                debug!("[makemkvcon] {}", m.message);
                            }
                            Some(SeverityLevel::Info) => {
                                info!("[makemkvcon] {}", m.message);
                            }
                            Some(SeverityLevel::Warning) => {
                                warn!("[makemkvcon] {}", m.message);
                                parsed_output.issues += 1;
                            }
                            Some(SeverityLevel::Error) => {
                                error!("[makemkvcon] {}", m.message);
                                parsed_output.errors += 1;
                            }
                            None => {
                                // unexpected msg code
                                warn!("[makemkvcon] MSG:{} {}", m.code, m.message);
                                parsed_output.issues += 1;
                            }
                        }
                        // special handling for MSG:1005
                        // MakeMKV v1.18.3 win(x64-release) started
                        if m.code == 1005 {
                            parsed_output.makemkv.version = m.params[0].clone();
                        }
                    }
                    ParsedOutputLine::DRV(d) => {
                        // skip empty slots "DriveStatus::NoDrive"
                        if d.drive_status_num != 256 {
                            parsed_output.drives.push(d);
                        };
                    }
                    ParsedOutputLine::TCOUNT(tc) => {
                        tc_want = tc;
                    }
                    ParsedOutputLine::CINFO(ir) => {
                        let (k, v) = convert_info_record(ir);
                        parsed_output.content.info.insert(k, v);
                    }
                    ParsedOutputLine::TINFO((tid, ir)) => {
                        // insert current title record into content record
                        let (attr, iro) = convert_info_record(ir);
                        match parsed_output.content.titles.get_mut(&tid) {
                            None => {
                                debug!("Parsing title {}.", tid);
                                let tr = TitleRecord {
                                    attributes: HashMap::from([(attr, iro)]),
                                    streams: HashMap::new(),
                                };
                                parsed_output.content.titles.insert(tid, tr);
                            }
                            Some(x) => {
                                x.attributes.insert(attr, iro);
                            }
                        }
                    }
                    ParsedOutputLine::SINFO((tid, sid, ir)) => {
                        // insert current stream record into title record
                        let (attr, iro) = convert_info_record(ir);
                        // Due to the way makemkvcon's output is generated
                        // it is safe to assume that the required title
                        // record already exists.
                        let tr = parsed_output.content.titles.get_mut(&tid).unwrap();
                        match tr.streams.get_mut(&sid) {
                            None => {
                                debug!("Parsing title {}, stream {}.", tid, sid);
                                let sr = StreamRecord {
                                    attributes: HashMap::from([(attr, iro)]),
                                };
                                tr.streams.insert(sid, sr);
                            }
                            Some(x) => {
                                x.attributes.insert(attr, iro);
                            }
                        }
                    }
                }
            }
            Err(e) => error!("Error reading line: {}", e),
        }
    }

    let tc_have = parsed_output.content.titles.len();
    if tc_have != tc_want {
        error!(
            "TCOUNT and actual title count differ! (TCOUNT: {}, titles: {})",
            tc_want, tc_have
        );
        parsed_output.errors += 1;
    }

    parsed_output
}

// validate the return type to ensure the match condition is working as
// expected (ignore the encapsulated value, it's already unit-tested)
mod tests {

    // cargo complains that 'use super::*' is unused but it's needed
    #[allow(unused_imports)]
    use super::*;

    #[test]
    fn test_parse_msg_record() {
        let data = b"MSG:1005,0,1,\"MakeMKV v1.17.9 linux(x64-release) started\",\"%1 started\",\"MakeMKV v1.17.9 linux(x64-release)\"";
        // ----------------------------------------------------------------
        // ----------------------------------------------------------------
        assert!(matches!(parse_output_line(data), ParsedOutputLine::MSG(_)));
    }

    #[test]
    fn test_parse_drv_record() {
        let data = b"DRV:0,0,999,0,\"DVD+R-DL PLDS DVD-RW DH16AFSH DL31 8SSDX0F17036L1CB5800MGJ\",\"\",\"/dev/sr1\"";
        // ----------------------------------------------------------------
        // ----------------------------------------------------------------
        assert!(matches!(parse_output_line(data), ParsedOutputLine::DRV(_)));
    }

    #[test]
    fn test_parse_tcount_record() {
        let data = b"TCOUNT:12";
        // ----------------------------------------------------------------
        // ----------------------------------------------------------------
        assert!(matches!(
            parse_output_line(data),
            ParsedOutputLine::TCOUNT(_)
        ));
    }

    #[test]
    fn test_parse_tinfo_record() {
        let data = b"TINFO:2,9,0,\"0:23:39\"";
        // ----------------------------------------------------------------
        // ----------------------------------------------------------------
        assert!(matches!(
            parse_output_line(data),
            ParsedOutputLine::TINFO(_)
        ));
    }

    // CINFO:1,6206,"DVD disc"
    #[test]
    fn test_parse_cinfo_record() {
        let data = b"CINFO:1,6206,\"DVD disc\"";
        // ----------------------------------------------------------------
        // ----------------------------------------------------------------
        assert!(matches!(
            parse_output_line(data),
            ParsedOutputLine::CINFO(_)
        ));
    }

    #[test]
    fn test_parse_sinfo_record() {
        let data = b"SINFO:2,0,1,6201,\"Video\"";
        // ----------------------------------------------------------------
        // ----------------------------------------------------------------
        assert!(matches!(
            parse_output_line(data),
            ParsedOutputLine::SINFO(_)
        ));
    }

    // cargo complains that these imports are unused but they are needed
    #[allow(unused_imports)]
    use crate::apdefs_h::DriveStatus;

    #[allow(unused_imports)]
    use crate::api::ContentType;

    #[test]
    fn test_parse_canned_output() {
        let data: Vec<&[u8]> = vec![
            b"MSG:1005,0,1,\"MakeMKV v1.17.9 linux(x64-release) started\",\"%1 started\",\"MakeMKV v1.17.9 linux(x64-release)\"",
            b"DRV:0,2,999,1,\"DVD+R-DL PLDS DVD-RW DH16AFSH DL31 8SSDX0F17036L1CB5800MGJ\",\"Disc 1\",\"/dev/sr0\"",
            b"TCOUNT:13",
        ];
        // ----------------------------------------------------------------
        let computed = parse_canned_output(&data).0;
        let expected = vec![
            DiscContent::DRV(DriveRecord {
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
                drive_name: "DVD+R-DL PLDS DVD-RW DH16AFSH DL31 8SSDX0F17036L1CB5800MGJ"
                    .to_string(),
                disc_name: "Disc 1".to_string(),
                device_name: "/dev/sr0".to_string(),
            }),
            DiscContent::TCOUNT(13),
        ];
        // ----------------------------------------------------------------
        assert_eq!(computed, expected);
    }

    #[test]
    fn test_parse_canned_output_with_errors() {
        // MSG after TCOUNT indicates an issue
        let data: Vec<&[u8]> = vec![
            b"MSG:1005,0,1,\"MakeMKV v1.17.9 linux(x64-release) started\",\"%1 started\",\"MakeMKV v1.17.9 linux(x64-release)\"",
            b"TCOUNT:13",
            b"MSG:9999,0,0,\"message\",\"message\"",
        ];
        // ----------------------------------------------------------------
        let computed = parse_canned_output(&data).1;
        let expected = 1;
        // ----------------------------------------------------------------
        assert_eq!(computed, expected);
    }
}

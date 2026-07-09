/*
    output parser for makemkvcon
*/

mod data_objects;

use std::collections::{BTreeMap, HashMap};

use crate::streams::{Stream, parse_stream_record};
use crate::{
    apdefs_h::ItemAttributeId,
    api::{
        DriveRecord, InfoRecord, MessageRecord, parse_content_info_data, parse_drive_record_data,
        parse_msg_data, parse_stream_info_data, parse_title_count_data, parse_title_info_data,
    },
    parse_as_usize,
};

#[allow(unused_imports)]
use log::{debug, error, info, warn};

pub use data_objects::{
    ContentAttributes, ContentRecord, InfoRecordOut, MakeMkvConfig, MakeMkvRecord, ParsedOutput,
    StreamAttributes, TitleAttributes, TitleRecord,
};

#[allow(clippy::upper_case_acronyms)]
#[derive(Debug, PartialEq)]
pub enum ParsedOutputLine {
    DRV(DriveRecord),
    MSG(MessageRecord),
    CINFO(InfoRecord),
    TINFO((usize, InfoRecord)),
    SINFO((usize, usize, InfoRecord)),
    TCOUNT(usize),
}

pub fn parse_output_line(line: &[u8]) -> ParsedOutputLine {
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

// ------------------------------------------------------------------------

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

// ------------------------------------------------------------------------

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
            info: ContentAttributes::default(),
            titles: BTreeMap::new(),
        },
        issues: 0,
        errors: 0,
    };
    let mut tc_want: usize = 0;
    let mut streams = HashMap::<usize, HashMap<usize, HashMap<u32, String>>>::new();
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
                        match ir.attr_val {
                            Some(ItemAttributeId::Comment) => {
                                parsed_output.content.info.comment = Some(ir.value);
                            }
                            Some(ItemAttributeId::Name) => {
                                parsed_output.content.info.name = Some(ir.value);
                            }
                            Some(ItemAttributeId::Type) => {
                                parsed_output.content.info.content_type = Some(ir.value);
                            }
                            Some(ItemAttributeId::MetadataLanguageCode) => {
                                parsed_output.content.info.metadata_language_code = Some(ir.value);
                            }
                            Some(ItemAttributeId::MetadataLanguageName) => {
                                parsed_output.content.info.metadata_language_name = Some(ir.value);
                            }
                            Some(ItemAttributeId::OrderWeight) => {
                                parsed_output.content.info.order_weight = parse_as_usize(&ir.value);
                            }
                            Some(ItemAttributeId::PanelTitle) => {
                                parsed_output.content.info.panel_title = Some(ir.value);
                            }
                            Some(ItemAttributeId::TreeInfo) => {
                                parsed_output.content.info.tree_info = Some(ir.value);
                            }
                            Some(ItemAttributeId::VolumeName) => {
                                parsed_output.content.info.volume_name = Some(ir.value);
                            }
                            // internal error
                            None => {
                                panic!(
                                    "Failed to process CINFO record. (id: {}, line: {})",
                                    ir.attr_num, line
                                );
                            }
                            // unknown value - unexpected makemkvcon output
                            _ => {
                                todo!(
                                    "Implement missing attribute '{}'. (line: {})",
                                    ir.attr_val.unwrap(),
                                    line
                                );
                            }
                        }
                    }
                    ParsedOutputLine::TINFO((tid, ir)) => {
                        // get TitleRecord reference
                        let tr = parsed_output.content.titles.entry(tid).or_default();
                        // update values
                        match ir.attr_val {
                            Some(ItemAttributeId::AngleInfo) => {
                                tr.info.angle_info = Some(ir.value);
                            }
                            Some(ItemAttributeId::ChapterCount) => {
                                // panics if 'ChapterCount' can't be parsed as a number
                                tr.info.chapter_count = parse_as_usize(&ir.value).unwrap();
                            }
                            Some(ItemAttributeId::Comment) => {
                                tr.info.comment = Some(ir.value);
                            }
                            Some(ItemAttributeId::DiskSize) => {
                                tr.info.disk_size = Some(ir.value);
                            }
                            Some(ItemAttributeId::DiskSizeBytes) => {
                                // panics if 'DiskSizeBytes' can't be parsed as a number
                                tr.info.disk_size_bytes = parse_as_usize(&ir.value).unwrap();
                            }
                            Some(ItemAttributeId::Duration) => {
                                tr.info.duration = Some(ir.value);
                            }
                            Some(ItemAttributeId::MetadataLanguageCode) => {
                                tr.info.metadata_language_code = Some(ir.value);
                            }
                            Some(ItemAttributeId::MetadataLanguageName) => {
                                tr.info.metadata_language_name = Some(ir.value);
                            }
                            Some(ItemAttributeId::Name) => {
                                tr.info.name = Some(ir.value);
                            }
                            Some(ItemAttributeId::OutputFileName) => {
                                tr.info.output_file_name = Some(ir.value);
                            }
                            Some(ItemAttributeId::OrderWeight) => {
                                // panics if 'OrderWeight' can't be parsed as a number
                                tr.info.order_weight = parse_as_usize(&ir.value).unwrap();
                            }
                            Some(ItemAttributeId::OriginalTitleId) => {
                                tr.info.original_title_id = Some(ir.value);
                            }
                            Some(ItemAttributeId::PanelTitle) => {
                                tr.info.panel_title = Some(ir.value);
                            }
                            Some(ItemAttributeId::SourceFileName) => {
                                tr.info.source_file_name = Some(ir.value);
                            }
                            Some(ItemAttributeId::SegmentsCount) => {
                                // panics if 'SegmentsCount' can't be parsed as a number
                                tr.info.segments_count = parse_as_usize(&ir.value).unwrap();
                            }
                            Some(ItemAttributeId::SegmentsMap) => {
                                tr.info.segments_map = Some(ir.value);
                            }
                            Some(ItemAttributeId::TreeInfo) => {
                                tr.info.tree_info = Some(ir.value);
                            }
                            // internal error
                            None => {
                                panic!(
                                    "Failed to process TINFO record. (id: {}, title: {}, line: {})",
                                    ir.attr_num, tid, line
                                );
                            }
                            // unknown value - unexpected makemkvcon output
                            _ => {
                                todo!(
                                    "Implement missing attribute '{}'. (title: {}, line: {})",
                                    ir.attr_val.unwrap(),
                                    tid,
                                    line
                                );
                            }
                        }
                    }
                    ParsedOutputLine::SINFO((tid, sid, ir)) => {
                        let tr = streams.entry(tid).or_default();
                        let sr = tr.entry(sid).or_default();
                        sr.insert(ir.attr_num, ir.value.clone());
                    }
                }
            }
            Err(e) => error!("Error reading line: {}", e),
        }
    }

    for (tid, title) in streams.iter() {
        for (sid, stream) in title.iter() {
            debug!("Processing title {} / stream {}.", tid, sid);
            match parse_stream_record(stream) {
                Some(Stream::Audio(x)) => {
                    parsed_output
                        .content
                        .titles
                        .entry(*tid)
                        .or_default()
                        .streams
                        .audio
                        .insert(*sid, x);
                }
                Some(Stream::Subtitle(x)) => {
                    parsed_output
                        .content
                        .titles
                        .entry(*tid)
                        .or_default()
                        .streams
                        .subtitles
                        .insert(*sid, x);
                }
                Some(Stream::Video(x)) => {
                    parsed_output
                        .content
                        .titles
                        .entry(*tid)
                        .or_default()
                        .streams
                        .video
                        .insert(*sid, x);
                }
                None => {
                    panic!("Detected an unknown stream type!");
                }
            }
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

    #[test]
    fn test_parse_all_data_files() {
        let data_dir = "tests/data";
        for entry in std::fs::read_dir(data_dir).unwrap() {
            let entry = entry.unwrap();
            let path = entry.path();
            warn!("Processing '{}'.", path.to_string_lossy());
            if path.extension().and_then(|s| s.to_str()) != Some("out") {
                continue;
            }

            let fh = std::fs::File::open(&path).unwrap();
            let reader = std::io::BufReader::new(fh);
            let min_length = 0;
            let severity_map = HashMap::new();

            let result = process_output(reader, min_length, &severity_map);

            // basic sanity checks: MakeMKV version was parsed and config preserved
            assert!(
                !result.makemkv.version.is_empty(),
                "empty MakeMKV version for {:?}",
                path
            );
            assert_eq!(result.makemkv.config.min_length, min_length);
        }
    }
}

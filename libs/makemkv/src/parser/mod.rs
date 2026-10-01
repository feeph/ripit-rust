/*!
    output parser for makemkvcon
*/

// standard library imports
// <none>

// third-party imports
#[allow(unused_imports)]
use log::{debug, error, info, warn};

// crate-provided imports
use crate::api::{
    DrvRecord, InfoRecord, MsgRecord, parse_cinfo_data, parse_drv_data, parse_msg_data,
    parse_prgc_data, parse_prgt_data, parse_prgv_data, parse_sinfo_data, parse_tcount_data,
    parse_tinfo_data,
};

// ------------------------------------------------------------------------
// public interface
// ------------------------------------------------------------------------

#[allow(clippy::upper_case_acronyms)]
#[derive(Debug, PartialEq)]
pub enum ParsedOutputLine {
    // messages
    MSG(MsgRecord),
    // drive-related events (includes medium info)
    DRV(DrvRecord),
    // content-related events
    TCOUNT(usize),
    CINFO(InfoRecord),
    TINFO((usize, InfoRecord)),
    SINFO((usize, usize, InfoRecord)),
    // progress-related events
    PRGT((u32, u32, String)),
    PRGC((u32, u32, String)),
    PRGV((u32, u32, u32)),
}

pub fn parse_output_line(line: &[u8]) -> ParsedOutputLine {
    // split input at first colon
    // "<id>:<data>" -> "<id>" and ":<data>"
    let idx = line.iter().position(|x| x == &b':').unwrap();
    let (id, data) = line.split_at(idx);

    match id {
        // skip the leading ':' in data
        b"MSG" => ParsedOutputLine::MSG(parse_msg_data(&data[1..])),
        b"DRV" => ParsedOutputLine::DRV(parse_drv_data(&data[1..])),
        b"TCOUNT" => ParsedOutputLine::TCOUNT(parse_tcount_data(&data[1..])),
        b"TINFO" => ParsedOutputLine::TINFO(parse_tinfo_data(&data[1..])),
        b"CINFO" => ParsedOutputLine::CINFO(parse_cinfo_data(&data[1..])),
        b"SINFO" => ParsedOutputLine::SINFO(parse_sinfo_data(&data[1..])),
        b"PRGC" => ParsedOutputLine::PRGC(parse_prgc_data(&data[1..])),
        b"PRGT" => ParsedOutputLine::PRGT(parse_prgt_data(&data[1..])),
        b"PRGV" => ParsedOutputLine::PRGV(parse_prgv_data(&data[1..])),
        _ => panic!("Found unsupported ID {}!", std::str::from_utf8(id).unwrap()),
    }
}

// ------------------------------------------------------------------------
// tests
// ------------------------------------------------------------------------

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
    fn test_parse_msg_record_with_quotes() {
        let data = b"MSG:5072,131072,1,\"Backing up disc into folder \\\"file:///media/backup/dump/_dev/DVDVolume_c08bef3b20202020.iso\\\"\",\"Backing up disc into folder \\\"%1\\\"\",\"file:///media/backup/dump/_dev/DVDVolume_c08bef3b20202020.iso\"";
        // ----------------------------------------------------------------
        let computed = match parse_output_line(data) {
            ParsedOutputLine::MSG(msg) => msg,
            _ => panic!("Not a message!"),
        };
        let expected = MsgRecord {
            code: 5072,
            flags: 131072,
            count: 1,
            message: "Backing up disc into folder \"file:///media/backup/dump/_dev/DVDVolume_c08bef3b20202020.iso\"".to_string(),
            format: "Backing up disc into folder \"%1\"".to_string(),
            params: Vec::from([
                "file:///media/backup/dump/_dev/DVDVolume_c08bef3b20202020.iso".to_string(),
            ]),
        };
        // ----------------------------------------------------------------
        assert_eq!(computed.message, expected.message);
        assert_eq!(computed.format, expected.format);
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

    // #[test]
    // fn test_parse_all_data_files() {
    //     let data_dir = "tests/data";
    //     for entry in std::fs::read_dir(data_dir).unwrap() {
    //         let entry = entry.unwrap();
    //         let path = entry.path();
    //         warn!("Processing '{}'.", path.to_string_lossy());
    //         if path.extension().and_then(|s| s.to_str()) != Some("out") {
    //             continue;
    //         }

    //         let fh = std::fs::File::open(&path).unwrap();
    //         let reader = std::io::BufReader::new(fh);
    //         let min_length = 0;
    //         let severity_map = HashMap::new();

    //         let result = process_output(reader, min_length, &severity_map);

    //         // basic sanity checks: MakeMKV version was parsed and config preserved
    //         assert!(
    //             !result.makemkv.version.is_empty(),
    //             "empty MakeMKV version for {:?}",
    //             path
    //         );
    //         assert_eq!(result.makemkv.config.min_length, min_length);
    //     }
    // }
}

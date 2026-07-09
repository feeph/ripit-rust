/*
    unit tests for 'process_output()'
    (parse output of makemkvcon --robot info <filename>)
*/

use std::collections::HashMap;

use makemkv::parser::{
    ContentAttributes, MakeMkvConfig, MakeMkvRecord, TitleAttributes, process_output,
};

use makemkv::{AudioStream, SubtitleStream, VideoStream};

mod tests {

    use super::*;

    #[test]
    fn test_parse_output_content_info() {
        let out_file = "tests/data/THOSE_WHO_HUNT_ELVES_1_3b97e7f85f5f5f4d.out";
        let fh = std::fs::File::open(out_file).unwrap();
        let reader = std::io::BufReader::new(fh);
        let min_length = 0;
        let severity_map = HashMap::new();
        // ----------------------------------------------------------------
        let computed = process_output(reader, min_length, &severity_map)
            .content
            .info;
        let expected = ContentAttributes {
            comment: None,
            content_type: Some("DVD disc".to_string()),
            metadata_language_code: Some("eng".to_string()),
            metadata_language_name: Some("English".to_string()),
            name: Some("Those Who Hunt Elves 1".to_string()),
            order_weight: Some(0),
            panel_title: Some("<b>Source information</b><br>".to_string()),
            tree_info: Some("Those Who Hunt Elves 1".to_string()),
            volume_name: Some("THOSE_WHO_HUNT_ELVES_1".to_string()),
        };
        // ----------------------------------------------------------------
        assert_eq!(computed, expected);
    }

    #[test]
    fn test_parse_output_content_info_with_comment() {
        let out_file = "tests/data/JUMANJI_THE_NEXT_LEVEL_8d7ac032ee4fb472.out";
        let fh = std::fs::File::open(out_file).unwrap();
        let reader = std::io::BufReader::new(fh);
        let min_length = 0;
        let severity_map = HashMap::new();
        // ----------------------------------------------------------------
        let computed = process_output(reader, min_length, &severity_map)
            .content
            .info;
        let expected = ContentAttributes {
            comment: Some("BD-133727_Jumanji_The_Next_Level_P6".to_string()),
            content_type: Some("Blu-ray disc".to_string()),
            metadata_language_code: Some("eng".to_string()),
            metadata_language_name: Some("English".to_string()),
            name: Some("Jumanji: The Next Level".to_string()),
            order_weight: Some(0),
            panel_title: Some("<b>Source information</b><br>".to_string()),
            tree_info: Some("Jumanji: The Next Level".to_string()),
            volume_name: None,
        };
        // ----------------------------------------------------------------
        assert_eq!(computed, expected);
    }

    #[test]
    fn test_parse_output_content_titles_attributes() {
        let out_file = "tests/data/THOSE_WHO_HUNT_ELVES_1_3b97e7f85f5f5f4d.out";
        let fh = std::fs::File::open(out_file).unwrap();
        let reader = std::io::BufReader::new(fh);
        let min_length = 0;
        let severity_map = HashMap::new();
        // ----------------------------------------------------------------
        let computed = process_output(reader, min_length, &severity_map)
            .content
            .titles[&0]
            .info
            .clone();
        let expected = TitleAttributes {
            angle_info: None,
            chapter_count: 5,
            comment: Some("B1".to_string()),
            disk_size: Some("1.1 GB".to_string()),
            disk_size_bytes: 1223931904,
            duration: Some("0:23:34".to_string()),
            metadata_language_code: Some("eng".to_string()),
            metadata_language_name: Some("English".to_string()),
            name: Some("Those Who Hunt Elves 1".to_string()),
            order_weight: 0,
            original_title_id: Some("02".to_string()),
            output_file_name: Some("Those Who Hunt Elves 1-B1_t00.mkv".to_string()),
            panel_title: Some("<b>Title information</b><br>".to_string()),
            segments_count: 1,
            segments_map: Some("1-5".to_string()),
            source_file_name: None,
            tree_info: Some("Those Who Hunt Elves 1 - 5 chapter(s) , 1.1 GB (B1)".to_string()),
        };
        // ----------------------------------------------------------------
        assert_eq!(computed, expected);
    }

    #[test]
    fn test_parse_output_content_titles_video_stream() {
        let out_file = "tests/data/THOSE_WHO_HUNT_ELVES_1_3b97e7f85f5f5f4d.out";
        let fh = std::fs::File::open(out_file).unwrap();
        let reader = std::io::BufReader::new(fh);
        let min_length = 0;
        let severity_map = HashMap::new();
        // ----------------------------------------------------------------
        let computed = process_output(reader, min_length, &severity_map)
            .content
            .titles[&0]
            .streams
            .video[&0]
            .clone();
        let expected = VideoStream {
            bitrate: "9.3 Mb/s".to_string(),
            codec_id: "V_MPEG2".to_string(),
            codec_long: "Mpeg2".to_string(),
            codec_short: "Mpeg2".to_string(),
            metadata_language_code: "eng".to_string(),
            metadata_language_name: "English".to_string(),
            mkv_flags: "".to_string(),
            order_weight: 0,
            output_conversion_type: "( Lossless conversion )".to_string(),
            panel_title: "<b>Track information</b><br>".to_string(),
            stream_flags: 0,
            stream_type: "Video".to_string(),
            tree_info: "Mpeg2".to_string(),
            video_aspect_ratio: "4:3".to_string(),
            video_frame_rate: "29.97 (30000/1001)".to_string(),
            video_size: "720x480".to_string(),
        };
        // ----------------------------------------------------------------
        assert_eq!(computed, expected);
    }

    #[test]
    fn test_parse_output_drives() {
        let out_file = "tests/data/THOSE_WHO_HUNT_ELVES_1_3b97e7f85f5f5f4d.out";
        let fh = std::fs::File::open(out_file).unwrap();
        let reader = std::io::BufReader::new(fh);
        let min_length = 0;
        let severity_map = HashMap::new();
        // ----------------------------------------------------------------
        let computed = process_output(reader, min_length, &severity_map).drives;
        let expected = Vec::new();
        // ----------------------------------------------------------------
        assert_eq!(computed, expected);
    }

    #[test]
    fn test_parse_output_errors() {
        let out_file = "tests/data/THOSE_WHO_HUNT_ELVES_1_3b97e7f85f5f5f4d.out";
        let fh = std::fs::File::open(out_file).unwrap();
        let reader = std::io::BufReader::new(fh);
        let min_length = 0;
        let severity_map = HashMap::new();
        // ----------------------------------------------------------------
        let computed = process_output(reader, min_length, &severity_map).errors;
        let expected = 0;
        // ----------------------------------------------------------------
        assert_eq!(computed, expected);
    }

    #[test]
    fn test_parse_output_issues() {
        let out_file = "tests/data/THOSE_WHO_HUNT_ELVES_1_3b97e7f85f5f5f4d.out";
        let fh = std::fs::File::open(out_file).unwrap();
        let reader = std::io::BufReader::new(fh);
        let min_length = 0;
        let severity_map = HashMap::new();
        // ----------------------------------------------------------------
        let computed = process_output(reader, min_length, &severity_map).issues;
        let expected = 15;
        // ----------------------------------------------------------------
        assert_eq!(computed, expected);
    }

    #[test]
    fn test_parse_output_makemkv() {
        let out_file = "tests/data/THOSE_WHO_HUNT_ELVES_1_3b97e7f85f5f5f4d.out";
        let fh = std::fs::File::open(out_file).unwrap();
        let reader = std::io::BufReader::new(fh);
        let min_length = 0;
        let severity_map = HashMap::new();
        // ----------------------------------------------------------------
        let computed = process_output(reader, min_length, &severity_map).makemkv;
        let expected = MakeMkvRecord {
            version: "MakeMKV v1.18.1 linux(x64-release)".to_string(),
            config: MakeMkvConfig { min_length },
        };
        // ----------------------------------------------------------------
        assert_eq!(computed, expected);
    }
}

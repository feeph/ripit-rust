use makemkvcon::parser::{
    InfoRecordOut, MakeMkvConfig, MakeMkvRecord, StreamRecord, process_output,
};

mod tests {
    use std::collections::HashMap;

    // cargo complains that 'use super::*' is unused but it's needed
    #[allow(unused_imports)]
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
        let expected = HashMap::from([
            (
                "MetadataLanguageCode".to_string(),
                InfoRecordOut {
                    code: "Unknown (0)".to_string(),
                    value: "eng".to_string(),
                },
            ),
            (
                "OrderWeight".to_string(),
                InfoRecordOut {
                    code: "Unknown (0)".to_string(),
                    value: "0".to_string(),
                },
            ),
            (
                "TreeInfo".to_string(),
                InfoRecordOut {
                    code: "Unknown (0)".to_string(),
                    value: "Those Who Hunt Elves 1".to_string(),
                },
            ),
            (
                "MetadataLanguageName".to_string(),
                InfoRecordOut {
                    code: "Unknown (0)".to_string(),
                    value: "English".to_string(),
                },
            ),
            (
                "VolumeName".to_string(),
                InfoRecordOut {
                    code: "Unknown (0)".to_string(),
                    value: "THOSE_WHO_HUNT_ELVES_1".to_string(),
                },
            ),
            (
                "Type".to_string(),
                InfoRecordOut {
                    code: "DvdTypeDisk".to_string(),
                    value: "DVD disc".to_string(),
                },
            ),
            (
                "PanelTitle".to_string(),
                InfoRecordOut {
                    code: "AppInterfaceItemInfoSource".to_string(),
                    value: "<b>Source information</b><br>".to_string(),
                },
            ),
            (
                "Name".to_string(),
                InfoRecordOut {
                    code: "Unknown (0)".to_string(),
                    value: "Those Who Hunt Elves 1".to_string(),
                },
            ),
        ]);
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
            .attributes
            .clone();
        let expected = HashMap::from([
            (
                "Name".to_string(),
                InfoRecordOut {
                    code: "Unknown (0)".to_string(),
                    value: "Those Who Hunt Elves 1".to_string(),
                },
            ),
            (
                "Comment".to_string(),
                InfoRecordOut {
                    code: "Unknown (0)".to_string(),
                    value: "B1".to_string(),
                },
            ),
            (
                "DiskSizeBytes".to_string(),
                InfoRecordOut {
                    code: "Unknown (0)".to_string(),
                    value: "1223931904".to_string(),
                },
            ),
            (
                "Duration".to_string(),
                InfoRecordOut {
                    code: "Unknown (0)".to_string(),
                    value: "0:23:34".to_string(),
                },
            ),
            (
                "PanelTitle".to_string(),
                InfoRecordOut {
                    code: "AppInterfaceItemInfoTitle".to_string(),
                    value: "<b>Title information</b><br>".to_string(),
                },
            ),
            (
                "MetadataLanguageCode".to_string(),
                InfoRecordOut {
                    code: "Unknown (0)".to_string(),
                    value: "eng".to_string(),
                },
            ),
            (
                "OrderWeight".to_string(),
                InfoRecordOut {
                    code: "Unknown (0)".to_string(),
                    value: "0".to_string(),
                },
            ),
            (
                "TreeInfo".to_string(),
                InfoRecordOut {
                    code: "Unknown (0)".to_string(),
                    value: "Those Who Hunt Elves 1 - 5 chapter(s) , 1.1 GB (B1)".to_string(),
                },
            ),
            (
                "MetadataLanguageName".to_string(),
                InfoRecordOut {
                    code: "Unknown (0)".to_string(),
                    value: "English".to_string(),
                },
            ),
            (
                "SegmentsMap".to_string(),
                InfoRecordOut {
                    code: "Unknown (0)".to_string(),
                    value: "1-5".to_string(),
                },
            ),
            (
                "DiskSize".to_string(),
                InfoRecordOut {
                    code: "Unknown (0)".to_string(),
                    value: "1.1 GB".to_string(),
                },
            ),
            (
                "ChapterCount".to_string(),
                InfoRecordOut {
                    code: "Unknown (0)".to_string(),
                    value: "5".to_string(),
                },
            ),
            (
                "OriginalTitleId".to_string(),
                InfoRecordOut {
                    code: "Unknown (0)".to_string(),
                    value: "02".to_string(),
                },
            ),
            (
                "SegmentsCount".to_string(),
                InfoRecordOut {
                    code: "Unknown (0)".to_string(),
                    value: "1".to_string(),
                },
            ),
            (
                "OutputFileName".to_string(),
                InfoRecordOut {
                    code: "Unknown (0)".to_string(),
                    value: "Those Who Hunt Elves 1-B1_t00.mkv".to_string(),
                },
            ),
        ]);

        // streams: {
        //     4: StreamRecord { attributes: {"LangCode": InfoRecordOut { code: "Unknown (0)", value: "eng" }, "Type": InfoRecordOut { code: "AppTitleTreeSubPicture", value: "Subtitles" }, "CodecLong": InfoRecordOut { code: "Unknown (0)", value: "Dvd Subtitles" }, "MetadataLanguageCode": InfoRecordOut { code: "Unknown (0)", value: "eng" }, "TreeInfo": InfoRecordOut { code: "Unknown (0)", value: " English" }, "MkvFlags": InfoRecordOut { code: "Unknown (0)", value: "d" }, "CodecShort": InfoRecordOut { code: "Unknown (0)", value: "" }, "MkvFlagsText": InfoRecordOut { code: "Unknown (0)", value: "Default" }, "OutputConversionType": InfoRecordOut { code: "Unknown (5088)", value: "( Lossless conversion )" }, "MetadataLanguageName": InfoRecordOut { code: "Unknown (0)", value: "English" }, "StreamFlags": InfoRecordOut { code: "Unknown (0)", value: "0" }, "LangName": InfoRecordOut { code: "Unknown (0)", value: "English" }, "CodecId": InfoRecordOut { code: "Unknown (0)", value: "S_VOBSUB" }, "PanelTitle": InfoRecordOut { code: "AppInterfaceItemInfoTRACK", value: "<b>Track information</b><br>" }, "OrderWeight": InfoRecordOut { code: "Unknown (0)", value: "90" }} }, 2: StreamRecord { attributes: {"CodecShort": InfoRecordOut { code: "Unknown (0)", value: "DD" }, "Name": InfoRecordOut { code: "Unknown (5091)", value: "Stereo" }, "LangCode": InfoRecordOut { code: "Unknown (0)", value: "jpn" }, "MetadataLanguageName": InfoRecordOut { code: "Unknown (0)", value: "English" }, "Type": InfoRecordOut { code: "AppTitleTreeAudio", value: "Audio" }, "MetadataLanguageCode": InfoRecordOut { code: "Unknown (0)", value: "eng" }, "LangName": InfoRecordOut { code: "Unknown (0)", value: "Japanese" }, "CodecId": InfoRecordOut { code: "Unknown (0)", value: "A_AC3" }, "CodecLong": InfoRecordOut { code: "Unknown (0)", value: "Dolby Digital" }, "AudioChannelsCount": InfoRecordOut { code: "Unknown (0)", value: "2" }, "AudioSampleRate": InfoRecordOut { code: "Unknown (0)", value: "48000" }, "PanelTitle": InfoRecordOut { code: "AppInterfaceItemInfoTRACK", value: "<b>Track information</b><br>" }, "OrderWeight": InfoRecordOut { code: "Unknown (0)", value: "90" }, "Bitrate": InfoRecordOut { code: "Unknown (0)", value: "224 Kb/s" }, "TreeInfo": InfoRecordOut { code: "Unknown (0)", value: "DD Stereo Japanese" }, "MkvFlags": InfoRecordOut { code: "Unknown (0)", value: "" }, "AudioChannelLayoutName": InfoRecordOut { code: "Unknown (0)", value: "stereo" }, "OutputConversionType": InfoRecordOut { code: "Unknown (5088)", value: "( Lossless conversion )" }, "StreamFlags": InfoRecordOut { code: "Unknown (0)", value: "0" }} }, 3: StreamRecord { attributes: {"LangCode": InfoRecordOut { code: "Unknown (0)", value: "spa" }, "CodecId": InfoRecordOut { code: "Unknown (0)", value: "A_AC3" }, "PanelTitle": InfoRecordOut { code: "AppInterfaceItemInfoTRACK", value: "<b>Track information</b><br>" }, "AudioChannelsCount": InfoRecordOut { code: "Unknown (0)", value: "2" }, "AudioChannelLayoutName": InfoRecordOut { code: "Unknown (0)", value: "stereo" }, "MkvFlags": InfoRecordOut { code: "Unknown (0)", value: "" }, "Type": InfoRecordOut { code: "AppTitleTreeAudio", value: "Audio" }, "Bitrate": InfoRecordOut { code: "Unknown (0)", value: "224 Kb/s" }, "AudioSampleRate": InfoRecordOut { code: "Unknown (0)", value: "48000" }, "LangName": InfoRecordOut { code: "Unknown (0)", value: "Spanish" }, "TreeInfo": InfoRecordOut { code: "Unknown (0)", value: "DD Stereo Spanish" }, "Name": InfoRecordOut { code: "Unknown (5091)", value: "Stereo" }, "OrderWeight": InfoRecordOut { code: "Unknown (0)", value: "90" }, "CodecShort": InfoRecordOut { code: "Unknown (0)", value: "DD" }, "OutputConversionType": InfoRecordOut { code: "Unknown (5088)", value: "( Lossless conversion )" }, "CodecLong": InfoRecordOut { code: "Unknown (0)", value: "Dolby Digital" }, "MetadataLanguageName": InfoRecordOut { code: "Unknown (0)", value: "English" }, "MetadataLanguageCode": InfoRecordOut { code: "Unknown (0)", value: "eng" }, "StreamFlags": InfoRecordOut { code: "Unknown (0)", value: "0" }} }, 5: StreamRecord { attributes: {"MkvFlags": InfoRecordOut { code: "Unknown (0)", value: "" }, "OutputConversionType": InfoRecordOut { code: "Unknown (5088)", value: "( Lossless conversion )" }, "MetadataLanguageName": InfoRecordOut { code: "Unknown (0)", value: "English" }, "Type": InfoRecordOut { code: "AppTitleTreeSubPicture", value: "Subtitles" }, "PanelTitle": InfoRecordOut { code: "AppInterfaceItemInfoTRACK", value: "<b>Track information</b><br>" }, "OrderWeight": InfoRecordOut { code: "Unknown (0)", value: "90" }, "LangName": InfoRecordOut { code: "Unknown (0)", value: "English" }, "CodecId": InfoRecordOut { code: "Unknown (0)", value: "S_VOBSUB" }, "CodecLong": InfoRecordOut { code: "Unknown (0)", value: "Dvd Subtitles" }, "LangCode": InfoRecordOut { code: "Unknown (0)", value: "eng" }, "StreamFlags": InfoRecordOut { code: "Unknown (0)", value: "0" }, "CodecShort": InfoRecordOut { code: "Unknown (0)", value: "" }, "MetadataLanguageCode": InfoRecordOut { code: "Unknown (0)", value: "eng" }, "TreeInfo": InfoRecordOut { code: "Unknown (0)", value: " English" }} }, 0: StreamRecord { attributes: {"Type": InfoRecordOut { code: "AppTitleTreeVideo", value: "Video" }, "TreeInfo": InfoRecordOut { code: "Unknown (0)", value: "Mpeg2" }, "MkvFlags": InfoRecordOut { code: "Unknown (0)", value: "" }, "StreamFlags": InfoRecordOut { code: "Unknown (0)", value: "0" }, "OrderWeight": InfoRecordOut { code: "Unknown (0)", value: "0" }, "Bitrate": InfoRecordOut { code: "Unknown (0)", value: "9.3 Mb/s" }, "CodecId": InfoRecordOut { code: "Unknown (0)", value: "V_MPEG2" }, "MetadataLanguageName": InfoRecordOut { code: "Unknown (0)", value: "English" }, "CodecLong": InfoRecordOut { code: "Unknown (0)", value: "Mpeg2" }, "VideoAspectRatio": InfoRecordOut { code: "Unknown (0)", value: "4:3" }, "MetadataLanguageCode": InfoRecordOut { code: "Unknown (0)", value: "eng" }, "PanelTitle": InfoRecordOut { code: "AppInterfaceItemInfoTRACK", value: "<b>Track information</b><br>" }, "VideoSize": InfoRecordOut { code: "Unknown (0)", value: "720x480" }, "OutputConversionType": InfoRecordOut { code: "Unknown (5088)", value: "( Lossless conversion )" }, "VideoFrameRate": InfoRecordOut { code: "Unknown (0)", value: "29.97 (30000/1001)" }, "CodecShort": InfoRecordOut { code: "Unknown (0)", value: "Mpeg2" }} }, 1: StreamRecord { attributes: {"PanelTitle": InfoRecordOut { code: "AppInterfaceItemInfoTRACK", value: "<b>Track information</b><br>" }, "OrderWeight": InfoRecordOut { code: "Unknown (0)", value: "90" }, "MkvFlags": InfoRecordOut { code: "Unknown (0)", value: "d" }, "Type": InfoRecordOut { code: "AppTitleTreeAudio", value: "Audio" }, "AudioChannelsCount": InfoRecordOut { code: "Unknown (0)", value: "2" }, "Name": InfoRecordOut { code: "Unknown (5091)", value: "Stereo" }, "AudioSampleRate": InfoRecordOut { code: "Unknown (0)", value: "48000" }, "MetadataLanguageCode": InfoRecordOut { code: "Unknown (0)", value: "eng" }, "AudioChannelLayoutName": InfoRecordOut { code: "Unknown (0)", value: "stereo" }, "CodecLong": InfoRecordOut { code: "Unknown (0)", value: "Dolby Digital" }, "LangCode": InfoRecordOut { code: "Unknown (0)", value: "eng" }, "CodecId": InfoRecordOut { code: "Unknown (0)", value: "A_AC3" }, "OutputConversionType": InfoRecordOut { code: "Unknown (5088)", value: "( Lossless conversion )" }, "CodecShort": InfoRecordOut { code: "Unknown (0)", value: "DD" }, "MetadataLanguageName": InfoRecordOut { code: "Unknown (0)", value: "English" }, "Bitrate": InfoRecordOut { code: "Unknown (0)", value: "224 Kb/s" }, "TreeInfo": InfoRecordOut { code: "Unknown (0)", value: "DD Stereo English" }, "MkvFlagsText": InfoRecordOut { code: "Unknown (0)", value: "Default" }, "StreamFlags": InfoRecordOut { code: "Unknown (0)", value: "0" }, "LangName": InfoRecordOut { code: "Unknown (0)", value: "English" }} }} }
        // ----------------------------------------------------------------
        assert_eq!(computed, expected);
    }

    #[test]
    fn test_parse_output_content_titles_streams() {
        let out_file = "tests/data/THOSE_WHO_HUNT_ELVES_1_3b97e7f85f5f5f4d.out";
        let fh = std::fs::File::open(out_file).unwrap();
        let reader = std::io::BufReader::new(fh);
        let min_length = 0;
        let severity_map = HashMap::new();
        // ----------------------------------------------------------------
        let computed = process_output(reader, min_length, &severity_map)
            .content
            .titles[&0]
            .streams[&0]
            .clone();
        let expected = StreamRecord {
            attributes: HashMap::from([
                (
                    "Type".to_string(),
                    InfoRecordOut {
                        code: "AppTitleTreeVideo".to_string(),
                        value: "Video".to_string(),
                    },
                ),
                (
                    "TreeInfo".to_string(),
                    InfoRecordOut {
                        code: "Unknown (0)".to_string(),
                        value: "Mpeg2".to_string(),
                    },
                ),
                (
                    "MkvFlags".to_string(),
                    InfoRecordOut {
                        code: "Unknown (0)".to_string(),
                        value: "".to_string(),
                    },
                ),
                (
                    "StreamFlags".to_string(),
                    InfoRecordOut {
                        code: "Unknown (0)".to_string(),
                        value: "0".to_string(),
                    },
                ),
                (
                    "OrderWeight".to_string(),
                    InfoRecordOut {
                        code: "Unknown (0)".to_string(),
                        value: "0".to_string(),
                    },
                ),
                (
                    "Bitrate".to_string(),
                    InfoRecordOut {
                        code: "Unknown (0)".to_string(),
                        value: "9.3 Mb/s".to_string(),
                    },
                ),
                (
                    "CodecId".to_string(),
                    InfoRecordOut {
                        code: "Unknown (0)".to_string(),
                        value: "V_MPEG2".to_string(),
                    },
                ),
                (
                    "MetadataLanguageName".to_string(),
                    InfoRecordOut {
                        code: "Unknown (0)".to_string(),
                        value: "English".to_string(),
                    },
                ),
                (
                    "CodecLong".to_string(),
                    InfoRecordOut {
                        code: "Unknown (0)".to_string(),
                        value: "Mpeg2".to_string(),
                    },
                ),
                (
                    "VideoAspectRatio".to_string(),
                    InfoRecordOut {
                        code: "Unknown (0)".to_string(),
                        value: "4:3".to_string(),
                    },
                ),
                (
                    "MetadataLanguageCode".to_string(),
                    InfoRecordOut {
                        code: "Unknown (0)".to_string(),
                        value: "eng".to_string(),
                    },
                ),
                (
                    "PanelTitle".to_string(),
                    InfoRecordOut {
                        code: "AppInterfaceItemInfoTRACK".to_string(),
                        value: "<b>Track information</b><br>".to_string(),
                    },
                ),
                (
                    "VideoSize".to_string(),
                    InfoRecordOut {
                        code: "Unknown (0)".to_string(),
                        value: "720x480".to_string(),
                    },
                ),
                (
                    "OutputConversionType".to_string(),
                    InfoRecordOut {
                        code: "Unknown (5088)".to_string(),
                        value: "( Lossless conversion )".to_string(),
                    },
                ),
                (
                    "VideoFrameRate".to_string(),
                    InfoRecordOut {
                        code: "Unknown (0)".to_string(),
                        value: "29.97 (30000/1001)".to_string(),
                    },
                ),
                (
                    "CodecShort".to_string(),
                    InfoRecordOut {
                        code: "Unknown (0)".to_string(),
                        value: "Mpeg2".to_string(),
                    },
                ),
            ]),
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

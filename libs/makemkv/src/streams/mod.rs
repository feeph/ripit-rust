#[allow(unused_imports)]
use log::{debug, error, info, warn};

mod audio_stream;
mod subtitle_stream;
mod video_stream;

use std::collections::HashMap;

pub use audio_stream::AudioStream;
pub use subtitle_stream::SubtitleStream;
pub use video_stream::VideoStream;

pub enum Stream {
    Audio(AudioStream),
    Subtitle(SubtitleStream),
    Video(VideoStream),
}

pub fn parse_stream_record(attributes: &HashMap<u32, String>) -> Option<Stream> {
    // to ensure all attributes provided by MakeMKV are handled:
    //   1. convert attributes into a HashMap
    //   2. drain the expected values
    //   3. check if there is anything left
    // The expected outcome is the HashMap being empty since all
    // available attributes have been consumed.

    let mut attrs = attributes.clone();

    // SINFO:0,24,1,6203,"Subtitles"
    let stream_type = attrs.remove(&1).unwrap();
    // SINFO:0,24,5,0,"S_HDMV/PGS"
    let codec_id = attrs.remove(&5).unwrap_or("<unknown>".to_string());
    // SINFO:0,24,6,0,"PGS"
    let codec_short = attrs.remove(&6).unwrap_or("<unknown>".to_string());
    // SINFO:0,24,7,0,"HDMV PGS Subtitles"
    let codec_long = attrs.remove(&7).unwrap_or("<unknown>".to_string());
    // SINFO:0,24,22,0,"6144"
    let stream_flags = parse_as_usize(&attrs.remove(&22).unwrap()).unwrap(); // may panic
    // SINFO:0,24,28,0,"eng"
    let metadata_language_code = attrs.remove(&28).unwrap_or("<unknown>".to_string());
    // SINFO:0,24,29,0,"English"
    let metadata_language_name = attrs.remove(&29).unwrap_or("<unknown>".to_string());

    // SINFO:0,24,31,6121,"<b>Track information</b><br>"
    let panel_title = attrs.remove(&31).unwrap_or("<unknown>".to_string());
    // SINFO:0,24,33,0,"90"
    let order_weight = parse_as_usize(&attrs.remove(&33).unwrap()).unwrap();
    // SINFO:0,24,38,0,""
    let mkv_flags = attrs.remove(&38).unwrap_or("<unknown>".to_string());
    // SINFO:0,24,42,5088,"( Lossless conversion )"
    let output_conversion_type = attrs.remove(&42).unwrap_or("<unknown>".to_string());

    // TreeInfo may have a leading space
    // (this is typically encountered on subtitle streams)
    let tree_info = attrs
        .remove(&30)
        .unwrap_or("<unknown>".to_string())
        .trim_start()
        .to_string();

    match stream_type.as_ref() {
        "Audio" => {
            // additional attributes for audio streams
            // -- required --
            // SINFO:0,1,1,6202,"Audio"
            // SINFO:0,1,2,0,"Surround 5.1"
            let name = attrs.remove(&2).unwrap_or("<unknown>".to_string());
            // SINFO:0,1,5,0,"A_DTS"
            // SINFO:0,1,6,0,"DTS"
            // SINFO:0,1,7,0,"DTS"
            // SINFO:0,1,13,0,"768 Kb/s"
            let bitrate = attrs.remove(&13).unwrap_or("<unknown>".to_string());
            // SINFO:0,1,14,0,"6"
            let audio_channels_count = parse_as_usize(&attrs.remove(&14).unwrap()).unwrap();
            // SINFO:0,1,17,0,"48000"
            let audio_sample_rate = parse_as_usize(&attrs.remove(&17).unwrap()).unwrap();
            // SINFO:0,1,22,0,"0"
            // SINFO:0,1,28,0,"eng"
            // SINFO:0,1,29,0,"English"
            // SINFO:0,1,30,0,"DTS Surround 5.1 Japanese"
            // SINFO:0,1,31,6121,"<b>Track information</b><br>"
            // SINFO:0,1,33,0,"90"
            // SINFO:0,1,38,0,"d"
            // SINFO:0,1,39,0,"Default"
            let mkv_flags_text = attrs.remove(&39).unwrap_or("".to_string());
            // SINFO:0,1,40,0,"5.1(side)"
            let audio_channel_layout_name = attrs.remove(&40).unwrap_or("<unknown>".to_string());
            // SINFO:0,1,42,5088,"( Lossless conversion )"
            // -- optional --
            // SINFO:0,1,3,0,"jpn"
            let lang_code = attrs.remove(&3).unwrap_or("<unknown>".to_string());
            // SINFO:0,1,4,0,"Japanese"
            let lang_name = attrs.remove(&4).unwrap_or("<unknown>".to_string());

            Some(Stream::Audio(AudioStream {
                audio_channel_layout_name,
                audio_channels_count,
                audio_sample_rate,
                bitrate,
                codec_id,
                codec_long,
                codec_short,
                lang_code,
                lang_name,
                metadata_language_code,
                metadata_language_name,
                mkv_flags_text,
                mkv_flags,
                name,
                order_weight,
                output_conversion_type,
                panel_title,
                stream_flags,
                stream_type,
                tree_info,
            }))
        }
        "Subtitles" => {
            // additional attributes for subtitle streams
            // -- required --
            // SINFO:0,9,1,6203,"Subtitles"
            // SINFO:0,9,3,0,"dan"
            let lang_code = attrs.remove(&3).unwrap_or("<unknown>".to_string());
            // SINFO:0,9,4,0,"Danish"
            let lang_name = attrs.remove(&4).unwrap_or("<unknown>".to_string());
            // SINFO:0,9,5,0,"S_HDMV/PGS"
            // SINFO:0,9,6,0,"PGS"
            // SINFO:0,9,7,0,"HDMV PGS Subtitles"
            // SINFO:0,9,22,0,"0"
            // SINFO:0,9,28,0,"eng"
            // SINFO:0,9,29,0,"English"
            // SINFO:0,9,30,0,"PGS Danish"
            // SINFO:0,9,31,6121,"<b>Track information</b><br>"
            // SINFO:0,9,33,0,"90"
            // SINFO:0,9,38,0,""
            // SINFO:0,9,42,5088,"( Lossless conversion )"
            // SINFO:0,9,1,6203,"Subtitles"
            // SINFO:0,9,3,0,"dan"
            // SINFO:0,9,4,0,"Danish"
            // -- optional --
            let mkv_flags_text = attrs.remove(&39).unwrap_or("".to_string());

            Some(Stream::Subtitle(SubtitleStream {
                codec_id,
                codec_long,
                codec_short,
                lang_code,
                lang_name,
                metadata_language_code,
                metadata_language_name,
                mkv_flags,
                mkv_flags_text,
                order_weight,
                output_conversion_type,
                panel_title,
                stream_flags,
                stream_type,
                tree_info,
            }))
        }
        "Video" => {
            // additional attributes for video streams
            // -- required --
            // SINFO:0,0,1,6201,"Video"
            // SINFO:0,0,5,0,"V_VC1"
            // SINFO:0,0,6,0,"VC-1"
            // SINFO:0,0,7,0,"VC-1"
            // SINFO:0,0,19,0,"1920x1080"
            let video_size = attrs.remove(&19).unwrap_or("<unknown>".to_string());
            // SINFO:0,0,20,0,"16:9"
            let video_aspect_ratio = attrs.remove(&20).unwrap_or("<unknown>".to_string());
            // SINFO:0,0,21,0,"23.976 (24000/1001)"
            let video_frame_rate = attrs.remove(&21).unwrap_or("<unknown>".to_string());
            // SINFO:0,0,22,0,"0"
            // SINFO:0,0,28,0,"eng"
            // SINFO:0,0,29,0,"English"
            // SINFO:0,0,30,0,"VC-1"
            // SINFO:0,0,31,6121,"<b>Track information</b><br>"
            // SINFO:0,0,33,0,"0"
            // SINFO:0,0,38,0,""
            // SINFO:0,0,42,5088,"( Lossless conversion )"

            let bitrate = attrs.remove(&13).unwrap_or("<unknown>".to_string());

            Some(Stream::Video(VideoStream {
                bitrate,
                codec_id,
                codec_long,
                codec_short,
                metadata_language_code,
                metadata_language_name,
                mkv_flags,
                order_weight,
                output_conversion_type,
                panel_title,
                stream_flags,
                stream_type,
                tree_info,
                video_aspect_ratio,
                video_frame_rate,
                video_size,
            }))
        }
        _ => None,
    }
}

fn parse_as_usize(value: &str) -> Option<usize> {
    match value.parse::<usize>() {
        Ok(x) => Some(x),
        Err(_) => {
            warn!("Unable to parse '{}' as usize!", value);
            None
        }
    }
}

mod tests {

    #[allow(unused_imports)]
    use super::*;

    // ====================================================================
    // audio stream
    // ====================================================================

    #[test]
    fn test_parse_audio_stream_minimal() {
        let data = HashMap::from([
            // SINFO:0,1,1,6202,"Audio"
            (1, "Audio".to_string()),
            // SINFO:0,1,2,0,"Surround 5.1"
            (2, "Surround 5.1".to_string()),
            // SINFO:0,1,5,0,"A_DTS"
            (5, "A_AC3".to_string()),
            // SINFO:0,1,6,0,"DTS"
            (6, "DD".to_string()),
            // SINFO:0,1,7,0,"DTS"
            (7, "Dolby Digital".to_string()),
            // SINFO:0,1,13,0,"768 Kb/s"
            (13, "768 Kb/s".to_string()),
            // SINFO:0,1,14,0,"6"
            (14, "6".to_string()),
            // SINFO:0,1,17,0,"48000"
            (17, "48000".to_string()),
            // SINFO:0,1,22,0,"0"
            (22, "0".to_string()),
            // SINFO:0,1,30,0,"DTS Surround 5.1 Japanese"
            (30, "DD Stereo English".to_string()),
            // SINFO:0,1,31,6121,"<b>Track information</b><br>"
            (31, "<b>Track information</b><br>".to_string()),
            // SINFO:0,1,33,0,"90"
            (33, "90".to_string()),
            // SINFO:0,1,38,0,"d"
            (38, "d".to_string()),
            // SINFO:0,1,39,0,"Default"
            (39, "Default".to_string()),
            // SINFO:0,1,40,0,"5.1(side)"
            (40, "5.1(side)".to_string()),
            // SINFO:0,1,42,5088,"( Lossless conversion )"
            (42, "( Lossless conversion )".to_string()),
        ]);
        // ----------------------------------------------------------------
        let computed = match parse_stream_record(&data) {
            Some(Stream::Audio(x)) => x,
            _ => panic!("Not an audio stream!"),
        };
        let expected = AudioStream {
            audio_channel_layout_name: "5.1(side)".to_string(),
            audio_channels_count: 6,
            audio_sample_rate: 48000,
            bitrate: "768 Kb/s".to_string(),
            codec_id: "A_AC3".to_string(),
            codec_long: "Dolby Digital".to_string(),
            codec_short: "DD".to_string(),
            lang_code: "<unknown>".to_string(),
            lang_name: "<unknown>".to_string(),
            metadata_language_code: "<unknown>".to_string(),
            metadata_language_name: "<unknown>".to_string(),
            mkv_flags: "d".to_string(),
            mkv_flags_text: "Default".to_string(),
            name: "Surround 5.1".to_string(),
            order_weight: 90,
            output_conversion_type: "( Lossless conversion )".to_string(),
            panel_title: "<b>Track information</b><br>".to_string(),
            stream_flags: 0,
            stream_type: "Audio".to_string(),
            tree_info: "DD Stereo English".to_string(),
        };
        // ----------------------------------------------------------------
        assert_eq!(computed, expected);
    }

    #[test]
    fn test_parse_audio_stream_with_lang() {
        let data = HashMap::from([
            // SINFO:0,1,1,6202,"Audio"
            (1, "Audio".to_string()),
            // SINFO:0,1,2,5091,"Stereo"
            (2, "Stereo".to_string()),
            // SINFO:0,1,3,0,"eng"
            (3, "eng".to_string()),
            // SINFO:0,1,4,0,"English"
            (4, "English".to_string()),
            // SINFO:0,1,5,0,"A_AC3"
            (5, "A_AC3".to_string()),
            // SINFO:0,1,6,0,"DD"
            (6, "DD".to_string()),
            // SINFO:0,1,7,0,"Dolby Digital"
            (7, "Dolby Digital".to_string()),
            // SINFO:0,1,13,0,"224 Kb/s"
            (13, "224 Kb/s".to_string()),
            // SINFO:0,1,14,0,"2"
            (14, "2".to_string()),
            // SINFO:0,1,17,0,"48000"
            (17, "48000".to_string()),
            // SINFO:0,1,22,0,"0"
            (22, "0".to_string()),
            // SINFO:0,1,30,0,"DD Stereo English"
            (30, "DD Stereo English".to_string()),
            // SINFO:0,1,31,6121,"<b>Track information</b><br>"
            (31, "<b>Track information</b><br>".to_string()),
            // SINFO:0,1,33,0,"90"
            (33, "90".to_string()),
            // SINFO:0,1,38,0,"d"
            (38, "d".to_string()),
            // SINFO:0,1,39,0,"Default"
            (39, "Default".to_string()),
            // SINFO:0,1,40,0,"stereo"
            (40, "stereo".to_string()),
            // SINFO:0,1,42,5088,"( Lossless conversion )"
            (42, "( Lossless conversion )".to_string()),
        ]);
        // ----------------------------------------------------------------
        let computed = match parse_stream_record(&data) {
            Some(Stream::Audio(x)) => x,
            _ => panic!("Not an audio stream!"),
        };
        let expected = AudioStream {
            audio_channel_layout_name: "stereo".to_string(),
            audio_channels_count: 2,
            audio_sample_rate: 48000,
            bitrate: "224 Kb/s".to_string(),
            codec_id: "A_AC3".to_string(),
            codec_long: "Dolby Digital".to_string(),
            codec_short: "DD".to_string(),
            lang_code: "eng".to_string(),
            lang_name: "English".to_string(),
            metadata_language_code: "<unknown>".to_string(),
            metadata_language_name: "<unknown>".to_string(),
            mkv_flags: "d".to_string(),
            mkv_flags_text: "Default".to_string(),
            name: "Stereo".to_string(),
            order_weight: 90,
            output_conversion_type: "( Lossless conversion )".to_string(),
            panel_title: "<b>Track information</b><br>".to_string(),
            stream_flags: 0,
            stream_type: "Audio".to_string(),
            tree_info: "DD Stereo English".to_string(),
        };
        // ----------------------------------------------------------------
        assert_eq!(computed, expected);
    }

    #[test]
    fn test_parse_audio_stream_with_meta_lang() {
        let data = HashMap::from([
            // SINFO:0,1,1,6202,"Audio"
            (1, "Audio".to_string()),
            // SINFO:0,1,2,0,"Surround 5.1"
            (2, "Surround 5.1".to_string()),
            // SINFO:0,1,5,0,"A_DTS"
            (5, "A_AC3".to_string()),
            // SINFO:0,1,6,0,"DTS"
            (6, "DD".to_string()),
            // SINFO:0,1,7,0,"DTS"
            (7, "Dolby Digital".to_string()),
            // SINFO:0,1,13,0,"768 Kb/s"
            (13, "768 Kb/s".to_string()),
            // SINFO:0,1,14,0,"6"
            (14, "6".to_string()),
            // SINFO:0,1,17,0,"48000"
            (17, "48000".to_string()),
            // SINFO:0,1,22,0,"0"
            (22, "0".to_string()),
            // SINFO:0,1,28,0,"eng"
            (28, "eng".to_string()),
            // SINFO:0,1,29,0,"English"
            (29, "English".to_string()),
            // SINFO:0,1,30,0,"DTS Surround 5.1 Japanese"
            (30, "DD Stereo English".to_string()),
            // SINFO:0,1,31,6121,"<b>Track information</b><br>"
            (31, "<b>Track information</b><br>".to_string()),
            // SINFO:0,1,33,0,"90"
            (33, "90".to_string()),
            // SINFO:0,1,38,0,"d"
            (38, "d".to_string()),
            // SINFO:0,1,39,0,"Default"
            (39, "Default".to_string()),
            // SINFO:0,1,40,0,"5.1(side)"
            (40, "5.1(side)".to_string()),
            // SINFO:0,1,42,5088,"( Lossless conversion )"
            (42, "( Lossless conversion )".to_string()),
        ]);
        // ----------------------------------------------------------------
        let computed = match parse_stream_record(&data) {
            Some(Stream::Audio(x)) => x,
            _ => panic!("Not an audio stream!"),
        };
        let expected = AudioStream {
            audio_channel_layout_name: "5.1(side)".to_string(),
            audio_channels_count: 6,
            audio_sample_rate: 48000,
            bitrate: "768 Kb/s".to_string(),
            codec_id: "A_AC3".to_string(),
            codec_long: "Dolby Digital".to_string(),
            codec_short: "DD".to_string(),
            lang_code: "<unknown>".to_string(),
            lang_name: "<unknown>".to_string(),
            metadata_language_code: "eng".to_string(),
            metadata_language_name: "English".to_string(),
            mkv_flags: "d".to_string(),
            mkv_flags_text: "Default".to_string(),
            name: "Surround 5.1".to_string(),
            order_weight: 90,
            output_conversion_type: "( Lossless conversion )".to_string(),
            panel_title: "<b>Track information</b><br>".to_string(),
            stream_flags: 0,
            stream_type: "Audio".to_string(),
            tree_info: "DD Stereo English".to_string(),
        };
        // ----------------------------------------------------------------
        assert_eq!(computed, expected);
    }

    // // ====================================================================
    // // subtitle stream
    // // ====================================================================

    // #[test]
    // fn test_parse_subtitle_stream_minimal() {
    //     let data = HashMap::from([
    //         (
    //             "CodecId".to_string(),
    //             InfoRecordOut::from("Unknown (0)", "S_VOBSUB"),
    //         ),
    //         (
    //             "CodecLong".to_string(),
    //             InfoRecordOut::from("Unknown (0)", "Dvd Subtitles"),
    //         ),
    //         (
    //             "CodecShort".to_string(),
    //             InfoRecordOut::from("Unknown (0)", ""),
    //         ),
    //         (
    //             "MkvFlags".to_string(),
    //             InfoRecordOut::from("Unknown (0)", "d"),
    //         ),
    //         (
    //             "MkvFlagsText".to_string(),
    //             InfoRecordOut::from("Unknown (0)", "Default"),
    //         ),
    //         (
    //             "OrderWeight".to_string(),
    //             InfoRecordOut::from("Unknown (0)", "90"),
    //         ),
    //         (
    //             "OutputConversionType".to_string(),
    //             InfoRecordOut::from("Unknown (5088)", "( Lossless conversion )"),
    //         ),
    //         (
    //             "PanelTitle".to_string(),
    //             InfoRecordOut::from("AppInterfaceItemInfoTRACK", "<b>Track information</b><br>"),
    //         ),
    //         (
    //             "StreamFlags".to_string(),
    //             InfoRecordOut::from("Unknown (0)", "0"),
    //         ),
    //         (
    //             "TreeInfo".to_string(),
    //             InfoRecordOut::from("Unknown (0)", "English"),
    //         ),
    //         (
    //             "Type".to_string(),
    //             InfoRecordOut::from("AppTitleTreeSubPicture", "Subtitles"),
    //         ),
    //     ]);
    //     // ----------------------------------------------------------------
    //     let computed = match parse_stream_record(&data) {
    //         Some(Stream::Subtitle(x)) => x,
    //         _ => panic!("Not an audio stream!"),
    //     };
    //     let expected = SubtitleStream {
    //         codec_id: "S_VOBSUB".to_string(),
    //         codec_long: "Dvd Subtitles".to_string(),
    //         codec_short: "".to_string(),
    //         lang_code: "<unknown>".to_string(),
    //         lang_name: "<unknown>".to_string(),
    //         metadata_language_code: "<unknown>".to_string(),
    //         metadata_language_name: "<unknown>".to_string(),
    //         mkv_flags: "d".to_string(),
    //         mkv_flags_text: "Default".to_string(),
    //         order_weight: 90,
    //         output_conversion_type: "( Lossless conversion )".to_string(),
    //         panel_title: "<b>Track information</b><br>".to_string(),
    //         stream_flags: 0,
    //         stream_type: "Subtitles".to_string(),
    //         tree_info: "English".to_string(),
    //     };
    //     // ----------------------------------------------------------------
    //     assert_eq!(computed, expected);
    // }

    // #[test]
    // fn test_parse_subtitle_stream_with_lang() {
    //     let data = HashMap::from([
    //         (
    //             "CodecId".to_string(),
    //             InfoRecordOut::from("Unknown (0)", "S_VOBSUB"),
    //         ),
    //         (
    //             "CodecLong".to_string(),
    //             InfoRecordOut::from("Unknown (0)", "Dvd Subtitles"),
    //         ),
    //         (
    //             "CodecShort".to_string(),
    //             InfoRecordOut::from("Unknown (0)", ""),
    //         ),
    //         (
    //             "LangCode".to_string(),
    //             InfoRecordOut::from("Unknown (0)", "eng"),
    //         ),
    //         (
    //             "LangName".to_string(),
    //             InfoRecordOut::from("Unknown (0)", "English"),
    //         ),
    //         (
    //             "MkvFlags".to_string(),
    //             InfoRecordOut::from("Unknown (0)", "d"),
    //         ),
    //         (
    //             "MkvFlagsText".to_string(),
    //             InfoRecordOut::from("Unknown (0)", "Default"),
    //         ),
    //         (
    //             "OrderWeight".to_string(),
    //             InfoRecordOut::from("Unknown (0)", "90"),
    //         ),
    //         (
    //             "OutputConversionType".to_string(),
    //             InfoRecordOut::from("Unknown (5088)", "( Lossless conversion )"),
    //         ),
    //         (
    //             "PanelTitle".to_string(),
    //             InfoRecordOut::from("AppInterfaceItemInfoTRACK", "<b>Track information</b><br>"),
    //         ),
    //         (
    //             "StreamFlags".to_string(),
    //             InfoRecordOut::from("Unknown (0)", "0"),
    //         ),
    //         (
    //             "TreeInfo".to_string(),
    //             InfoRecordOut::from("Unknown (0)", "English"),
    //         ),
    //         (
    //             "Type".to_string(),
    //             InfoRecordOut::from("AppTitleTreeSubPicture", "Subtitles"),
    //         ),
    //     ]);
    //     // ----------------------------------------------------------------
    //     let computed = match parse_stream_record(&data) {
    //         Some(Stream::Subtitle(x)) => x,
    //         _ => panic!("Not an audio stream!"),
    //     };
    //     let expected = SubtitleStream {
    //         codec_id: "S_VOBSUB".to_string(),
    //         codec_long: "Dvd Subtitles".to_string(),
    //         codec_short: "".to_string(),
    //         lang_code: "eng".to_string(),
    //         lang_name: "English".to_string(),
    //         metadata_language_code: "<unknown>".to_string(),
    //         metadata_language_name: "<unknown>".to_string(),
    //         mkv_flags: "d".to_string(),
    //         mkv_flags_text: "Default".to_string(),
    //         order_weight: 90,
    //         output_conversion_type: "( Lossless conversion )".to_string(),
    //         panel_title: "<b>Track information</b><br>".to_string(),
    //         stream_flags: 0,
    //         stream_type: "Subtitles".to_string(),
    //         tree_info: "English".to_string(),
    //     };
    //     // ----------------------------------------------------------------
    //     assert_eq!(computed, expected);
    // }

    // #[test]
    // fn test_parse_subtitle_stream_with_meta_lang() {
    //     let data = HashMap::from([
    //         (
    //             "CodecId".to_string(),
    //             InfoRecordOut::from("Unknown (0)", "S_VOBSUB"),
    //         ),
    //         (
    //             "CodecLong".to_string(),
    //             InfoRecordOut::from("Unknown (0)", "Dvd Subtitles"),
    //         ),
    //         (
    //             "CodecShort".to_string(),
    //             InfoRecordOut::from("Unknown (0)", ""),
    //         ),
    //         (
    //             "MetadataLanguageCode".to_string(),
    //             InfoRecordOut::from("Unknown (0)", "eng"),
    //         ),
    //         (
    //             "MetadataLanguageName".to_string(),
    //             InfoRecordOut::from("Unknown (0)", "English"),
    //         ),
    //         (
    //             "MkvFlags".to_string(),
    //             InfoRecordOut::from("Unknown (0)", "d"),
    //         ),
    //         (
    //             "MkvFlagsText".to_string(),
    //             InfoRecordOut::from("Unknown (0)", "Default"),
    //         ),
    //         (
    //             "OrderWeight".to_string(),
    //             InfoRecordOut::from("Unknown (0)", "90"),
    //         ),
    //         (
    //             "OutputConversionType".to_string(),
    //             InfoRecordOut::from("Unknown (5088)", "( Lossless conversion )"),
    //         ),
    //         (
    //             "PanelTitle".to_string(),
    //             InfoRecordOut::from("AppInterfaceItemInfoTRACK", "<b>Track information</b><br>"),
    //         ),
    //         (
    //             "StreamFlags".to_string(),
    //             InfoRecordOut::from("Unknown (0)", "0"),
    //         ),
    //         (
    //             "TreeInfo".to_string(),
    //             InfoRecordOut::from("Unknown (0)", "English"),
    //         ),
    //         (
    //             "Type".to_string(),
    //             InfoRecordOut::from("AppTitleTreeSubPicture", "Subtitles"),
    //         ),
    //     ]);
    //     // ----------------------------------------------------------------
    //     let computed = match parse_stream_record(&data) {
    //         Some(Stream::Subtitle(x)) => x,
    //         _ => panic!("Not an audio stream!"),
    //     };
    //     let expected = SubtitleStream {
    //         codec_id: "S_VOBSUB".to_string(),
    //         codec_long: "Dvd Subtitles".to_string(),
    //         codec_short: "".to_string(),
    //         lang_code: "<unknown>".to_string(),
    //         lang_name: "<unknown>".to_string(),
    //         metadata_language_code: "eng".to_string(),
    //         metadata_language_name: "English".to_string(),
    //         mkv_flags: "d".to_string(),
    //         mkv_flags_text: "Default".to_string(),
    //         order_weight: 90,
    //         output_conversion_type: "( Lossless conversion )".to_string(),
    //         panel_title: "<b>Track information</b><br>".to_string(),
    //         stream_flags: 0,
    //         stream_type: "Subtitles".to_string(),
    //         tree_info: "English".to_string(),
    //     };
    //     // ----------------------------------------------------------------
    //     assert_eq!(computed, expected);
    // }

    // // ====================================================================
    // // video stream
    // // ====================================================================

    // #[test]
    // fn test_parse_video_stream_minimal() {
    //     let data = HashMap::from([
    //         (
    //             "Bitrate".to_string(),
    //             InfoRecordOut::from("Unknown (0)", "6 Mb/s"),
    //         ),
    //         (
    //             "CodecId".to_string(),
    //             InfoRecordOut::from("Unknown (0)", "V_MPEG2"),
    //         ),
    //         (
    //             "CodecLong".to_string(),
    //             InfoRecordOut::from("Unknown (0)", "Mpeg2"),
    //         ),
    //         (
    //             "CodecShort".to_string(),
    //             InfoRecordOut::from("Unknown (0)", "Mpeg2"),
    //         ),
    //         (
    //             "MkvFlags".to_string(),
    //             InfoRecordOut::from("Unknown (0)", ""),
    //         ),
    //         (
    //             "OrderWeight".to_string(),
    //             InfoRecordOut::from("Unknown (0)", "0"),
    //         ),
    //         (
    //             "OutputConversionType".to_string(),
    //             InfoRecordOut::from("Unknown (5088)", "( Lossless conversion )"),
    //         ),
    //         (
    //             "PanelTitle".to_string(),
    //             InfoRecordOut::from("AppInterfaceItemInfoTRACK", "<b>Track information</b><br>"),
    //         ),
    //         (
    //             "StreamFlags".to_string(),
    //             InfoRecordOut::from("Unknown (0)", "0"),
    //         ),
    //         (
    //             "TreeInfo".to_string(),
    //             InfoRecordOut::from("Unknown (0)", "Mpeg2"),
    //         ),
    //         (
    //             "Type".to_string(),
    //             InfoRecordOut::from("AppTitleTreeVideo", "Video"),
    //         ),
    //         (
    //             "VideoAspectRatio".to_string(),
    //             InfoRecordOut::from("Unknown (0)", "4:3"),
    //         ),
    //         (
    //             "VideoFrameRate".to_string(),
    //             InfoRecordOut::from("Unknown (0)", "29.97 (30000/1001)"),
    //         ),
    //         (
    //             "VideoSize".to_string(),
    //             InfoRecordOut::from("Unknown (0)", "720x480"),
    //         ),
    //     ]);
    //     // ----------------------------------------------------------------
    //     let computed = match parse_stream_record(&data) {
    //         Some(Stream::Video(x)) => x,
    //         _ => panic!("Not an audio stream!"),
    //     };
    //     let expected = VideoStream {
    //         bitrate: "6 Mb/s".to_string(),
    //         codec_id: "V_MPEG2".to_string(),
    //         codec_long: "Mpeg2".to_string(),
    //         codec_short: "Mpeg2".to_string(),
    //         metadata_language_code: "<unknown>".to_string(),
    //         metadata_language_name: "<unknown>".to_string(),
    //         mkv_flags: "".to_string(),
    //         order_weight: 0,
    //         output_conversion_type: "( Lossless conversion )".to_string(),
    //         panel_title: "<b>Track information</b><br>".to_string(),
    //         stream_flags: 0,
    //         stream_type: "Video".to_string(),
    //         tree_info: "Mpeg2".to_string(),
    //         video_aspect_ratio: "4:3".to_string(),
    //         video_frame_rate: "29.97 (30000/1001)".to_string(),
    //         video_size: "720x480".to_string(),
    //     };
    //     // ----------------------------------------------------------------
    //     assert_eq!(computed, expected);
    // }

    // #[test]
    // fn test_parse_video_stream_with_meta_lang() {
    //     let data = HashMap::from([
    //         (
    //             "Bitrate".to_string(),
    //             InfoRecordOut::from("Unknown (0)", "6 Mb/s"),
    //         ),
    //         (
    //             "CodecId".to_string(),
    //             InfoRecordOut::from("Unknown (0)", "V_MPEG2"),
    //         ),
    //         (
    //             "CodecLong".to_string(),
    //             InfoRecordOut::from("Unknown (0)", "Mpeg2"),
    //         ),
    //         (
    //             "CodecShort".to_string(),
    //             InfoRecordOut::from("Unknown (0)", "Mpeg2"),
    //         ),
    //         (
    //             "MetadataLanguageCode".to_string(),
    //             InfoRecordOut::from("Unknown (0)", "eng"),
    //         ),
    //         (
    //             "MetadataLanguageName".to_string(),
    //             InfoRecordOut::from("Unknown (0)", "English"),
    //         ),
    //         (
    //             "MkvFlags".to_string(),
    //             InfoRecordOut::from("Unknown (0)", ""),
    //         ),
    //         (
    //             "OrderWeight".to_string(),
    //             InfoRecordOut::from("Unknown (0)", "0"),
    //         ),
    //         (
    //             "OutputConversionType".to_string(),
    //             InfoRecordOut::from("Unknown (5088)", "( Lossless conversion )"),
    //         ),
    //         (
    //             "PanelTitle".to_string(),
    //             InfoRecordOut::from("AppInterfaceItemInfoTRACK", "<b>Track information</b><br>"),
    //         ),
    //         (
    //             "StreamFlags".to_string(),
    //             InfoRecordOut::from("Unknown (0)", "0"),
    //         ),
    //         (
    //             "TreeInfo".to_string(),
    //             InfoRecordOut::from("Unknown (0)", "Mpeg2"),
    //         ),
    //         (
    //             "Type".to_string(),
    //             InfoRecordOut::from("AppTitleTreeVideo", "Video"),
    //         ),
    //         (
    //             "VideoAspectRatio".to_string(),
    //             InfoRecordOut::from("Unknown (0)", "4:3"),
    //         ),
    //         (
    //             "VideoFrameRate".to_string(),
    //             InfoRecordOut::from("Unknown (0)", "29.97 (30000/1001)"),
    //         ),
    //         (
    //             "VideoSize".to_string(),
    //             InfoRecordOut::from("Unknown (0)", "720x480"),
    //         ),
    //     ]);
    //     // ----------------------------------------------------------------
    //     let computed = match parse_stream_record(&data) {
    //         Some(Stream::Video(x)) => x,
    //         _ => panic!("Not an audio stream!"),
    //     };
    //     let expected = VideoStream {
    //         bitrate: "6 Mb/s".to_string(),
    //         codec_id: "V_MPEG2".to_string(),
    //         codec_long: "Mpeg2".to_string(),
    //         codec_short: "Mpeg2".to_string(),
    //         metadata_language_code: "eng".to_string(),
    //         metadata_language_name: "English".to_string(),
    //         mkv_flags: "".to_string(),
    //         order_weight: 0,
    //         output_conversion_type: "( Lossless conversion )".to_string(),
    //         panel_title: "<b>Track information</b><br>".to_string(),
    //         stream_flags: 0,
    //         stream_type: "Video".to_string(),
    //         tree_info: "Mpeg2".to_string(),
    //         video_aspect_ratio: "4:3".to_string(),
    //         video_frame_rate: "29.97 (30000/1001)".to_string(),
    //         video_size: "720x480".to_string(),
    //     };
    //     // ----------------------------------------------------------------
    //     assert_eq!(computed, expected);
    // }
}

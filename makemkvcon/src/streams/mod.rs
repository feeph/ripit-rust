#[allow(unused_imports)]
use log::{debug, error, info, warn};

mod audio_stream;
mod subtitle_stream;
mod video_stream;

use std::collections::HashMap;

use crate::parser::InfoRecordOut;

pub use audio_stream::AudioStream;
pub use subtitle_stream::SubtitleStream;
pub use video_stream::VideoStream;

pub enum Stream {
    Audio(AudioStream),
    Subtitle(SubtitleStream),
    Video(VideoStream),
}

pub fn parse_stream_record(attributes: HashMap<String, InfoRecordOut>) -> Stream {
    // to ensure all attributes provided by MakeMKV are handled:
    //   1. convert attributes into a HashMap
    //   2. drain the expected values
    //   3. check if there is anything left
    // The expected outcome is the HashMap being empty since all
    // available attributes have been consumed.

    let mut attrs: HashMap<String, String> = HashMap::new();
    for (attr_name, attr_record) in attributes {
        attrs.insert(attr_name.clone(), attr_record.value.clone());
    }

    let codec_id = attrs.remove("CodecId").unwrap_or("<unknown>".to_string());
    let codec_long = attrs.remove("CodecLong").unwrap_or("<unknown>".to_string());
    let codec_short = attrs
        .remove("CodecShort")
        .unwrap_or("<unknown>".to_string());
    let metadata_language_code = attrs
        .remove("MetadataLanguageCode")
        .unwrap_or("<unknown>".to_string());
    let metadata_language_name = attrs
        .remove("MetadataLanguageName")
        .unwrap_or("<unknown>".to_string());
    let mkv_flags = attrs.remove("MkvFlags").unwrap_or("<unknown>".to_string());
    let order_weight_str = attrs.remove("OrderWeight").unwrap(); // may panic
    let order_weight = parse_as_usize(&order_weight_str).unwrap();
    let output_conversion_type = attrs
        .remove("OutputConversionType")
        .unwrap_or("<unknown>".to_string());
    let panel_title = attrs
        .remove("PanelTitle")
        .unwrap_or("<unknown>".to_string());
    let stream_flags = attrs.remove("StreamFlags").unwrap(); // may panic
    let stream_flags = parse_as_usize(&stream_flags).unwrap();
    let stream_type = attrs.remove("Type").unwrap();
    let stream_type_cpy = stream_type.clone();
    // TreeInfo may have a leading space
    // (this is typically encountered on subtitle streams)
    let tree_info = attrs
        .remove("TreeInfo")
        .unwrap_or("<unknown>".to_string())
        .trim_start()
        .to_string();

    let stream = match stream_type.as_ref() {
        "Audio" => {
            // additional attributes for audio streams
            let audio_channel_layout_name = attrs
                .remove("AudioChannelLayoutName")
                .unwrap_or("<unknown>".to_string());
            let audio_channels_count_str = attrs.remove("AudioChannelsCount").unwrap(); // may panic
            let audio_channels_count = parse_as_usize(&audio_channels_count_str).unwrap();
            let audio_sample_rate_str = attrs.remove("AudioSampleRate").unwrap(); // may panic
            let audio_sample_rate = parse_as_usize(&audio_sample_rate_str).unwrap();
            let bitrate = attrs.remove("Bitrate").unwrap_or("<unknown>".to_string());
            // "LangCode" may be missing
            let lang_code = attrs.remove("LangCode").unwrap_or("<unknown>".to_string());
            // "LangName" may be missing
            let lang_name = attrs.remove("LangName").unwrap_or("<unknown>".to_string());
            let mkv_flags_text = attrs.remove("MkvFlagsText").unwrap_or("".to_string());
            let name = attrs.remove("Name").unwrap_or("<unknown>".to_string());

            Stream::Audio(AudioStream {
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
                mkv_flags,
                mkv_flags_text,
                name,
                order_weight,
                output_conversion_type,
                panel_title,
                stream_flags,
                stream_type,
                tree_info,
            })
        }
        "Subtitles" => {
            // additional attributes for subtitle streams
            let lang_code = attrs.remove("LangCode").unwrap_or("<unknown>".to_string());
            let lang_name = attrs.remove("LangName").unwrap_or("<unknown>".to_string());
            let mkv_flags_text = attrs.remove("MkvFlagsText").unwrap_or("".to_string());

            Stream::Subtitle(SubtitleStream {
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
            })
        }
        "Video" => {
            // additional attributes for video streams
            let bitrate = attrs.remove("Bitrate").unwrap_or("<unknown>".to_string());
            let video_aspect_ratio = attrs
                .remove("VideoAspectRatio")
                .unwrap_or("<unknown>".to_string());
            let video_framerate = attrs
                .remove("VideoFrameRate")
                .unwrap_or("<unknown>".to_string());
            let video_size = attrs.remove("VideoSize").unwrap_or("<unknown>".to_string());

            Stream::Video(VideoStream {
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
                video_framerate,
                video_size,
            })
        }
        _ => {
            // must be an audio, subtitle or video stream
            panic!("Found unknown stream type '{}'!", stream_type);
        }
    };

    if !attrs.is_empty() {
        for (k, _) in attrs {
            warn!(
                "Found unexpected attribute {} in {} record!",
                k, stream_type_cpy
            );
        }
    }

    stream
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
            (
                "AudioChannelLayoutName".to_string(),
                InfoRecordOut::from("Unknown (0)", "stereo"),
            ),
            (
                "AudioChannelsCount".to_string(),
                InfoRecordOut::from("Unknown (0)", "2"),
            ),
            (
                "AudioSampleRate".to_string(),
                InfoRecordOut::from("Unknown (0)", "48000"),
            ),
            (
                "Bitrate".to_string(),
                InfoRecordOut::from("Unknown (0)", "224 Kb/s"),
            ),
            (
                "CodecId".to_string(),
                InfoRecordOut::from("Unknown (0)", "A_AC3"),
            ),
            (
                "CodecLong".to_string(),
                InfoRecordOut::from("Unknown (0)", "Dolby Digital"),
            ),
            (
                "CodecShort".to_string(),
                InfoRecordOut::from("Unknown (0)", "DD"),
            ),
            (
                "MkvFlags".to_string(),
                InfoRecordOut::from("Unknown (0)", "d"),
            ),
            (
                "MkvFlagsText".to_string(),
                InfoRecordOut::from("Unknown (0)", "Default"),
            ),
            (
                "Name".to_string(),
                InfoRecordOut::from("Unknown (5091)", "Stereo"),
            ),
            (
                "OrderWeight".to_string(),
                InfoRecordOut::from("Unknown (0)", "90"),
            ),
            (
                "OutputConversionType".to_string(),
                InfoRecordOut::from("Unknown (5088)", "( Lossless conversion )"),
            ),
            (
                "PanelTitle".to_string(),
                InfoRecordOut::from("AppInterfaceItemInfoTRACK", "<b>Track information</b><br>"),
            ),
            (
                "StreamFlags".to_string(),
                InfoRecordOut::from("Unknown (0)", "0"),
            ),
            (
                "TreeInfo".to_string(),
                InfoRecordOut::from("Unknown (0)", "DD Stereo English"),
            ),
            (
                "Type".to_string(),
                InfoRecordOut::from("AppTitleTreeAudio", "Audio"),
            ),
        ]);
        // ----------------------------------------------------------------
        let computed = match parse_stream_record(data) {
            Stream::Audio(x) => x,
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
            lang_code: "<unknown>".to_string(),
            lang_name: "<unknown>".to_string(),
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
    fn test_parse_audio_stream_with_lang() {
        let data = HashMap::from([
            (
                "AudioChannelLayoutName".to_string(),
                InfoRecordOut::from("Unknown (0)", "stereo"),
            ),
            (
                "AudioChannelsCount".to_string(),
                InfoRecordOut::from("Unknown (0)", "2"),
            ),
            (
                "AudioSampleRate".to_string(),
                InfoRecordOut::from("Unknown (0)", "48000"),
            ),
            (
                "Bitrate".to_string(),
                InfoRecordOut::from("Unknown (0)", "224 Kb/s"),
            ),
            (
                "CodecId".to_string(),
                InfoRecordOut::from("Unknown (0)", "A_AC3"),
            ),
            (
                "CodecLong".to_string(),
                InfoRecordOut::from("Unknown (0)", "Dolby Digital"),
            ),
            (
                "CodecShort".to_string(),
                InfoRecordOut::from("Unknown (0)", "DD"),
            ),
            (
                "LangCode".to_string(),
                InfoRecordOut::from("Unknown (0)", "eng"),
            ),
            (
                "LangName".to_string(),
                InfoRecordOut::from("Unknown (0)", "English"),
            ),
            (
                "MkvFlags".to_string(),
                InfoRecordOut::from("Unknown (0)", "d"),
            ),
            (
                "MkvFlagsText".to_string(),
                InfoRecordOut::from("Unknown (0)", "Default"),
            ),
            (
                "Name".to_string(),
                InfoRecordOut::from("Unknown (5091)", "Stereo"),
            ),
            (
                "OrderWeight".to_string(),
                InfoRecordOut::from("Unknown (0)", "90"),
            ),
            (
                "OutputConversionType".to_string(),
                InfoRecordOut::from("Unknown (5088)", "( Lossless conversion )"),
            ),
            (
                "PanelTitle".to_string(),
                InfoRecordOut::from("AppInterfaceItemInfoTRACK", "<b>Track information</b><br>"),
            ),
            (
                "StreamFlags".to_string(),
                InfoRecordOut::from("Unknown (0)", "0"),
            ),
            (
                "TreeInfo".to_string(),
                InfoRecordOut::from("Unknown (0)", "DD Stereo English"),
            ),
            (
                "Type".to_string(),
                InfoRecordOut::from("AppTitleTreeAudio", "Audio"),
            ),
        ]);
        // ----------------------------------------------------------------
        let computed = match parse_stream_record(data) {
            Stream::Audio(x) => x,
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
            (
                "AudioChannelLayoutName".to_string(),
                InfoRecordOut::from("Unknown (0)", "stereo"),
            ),
            (
                "AudioChannelsCount".to_string(),
                InfoRecordOut::from("Unknown (0)", "2"),
            ),
            (
                "AudioSampleRate".to_string(),
                InfoRecordOut::from("Unknown (0)", "48000"),
            ),
            (
                "Bitrate".to_string(),
                InfoRecordOut::from("Unknown (0)", "224 Kb/s"),
            ),
            (
                "CodecId".to_string(),
                InfoRecordOut::from("Unknown (0)", "A_AC3"),
            ),
            (
                "CodecLong".to_string(),
                InfoRecordOut::from("Unknown (0)", "Dolby Digital"),
            ),
            (
                "CodecShort".to_string(),
                InfoRecordOut::from("Unknown (0)", "DD"),
            ),
            (
                "MetadataLanguageCode".to_string(),
                InfoRecordOut::from("Unknown (0)", "eng"),
            ),
            (
                "MetadataLanguageName".to_string(),
                InfoRecordOut::from("Unknown (0)", "English"),
            ),
            (
                "MkvFlags".to_string(),
                InfoRecordOut::from("Unknown (0)", "d"),
            ),
            (
                "MkvFlagsText".to_string(),
                InfoRecordOut::from("Unknown (0)", "Default"),
            ),
            (
                "Name".to_string(),
                InfoRecordOut::from("Unknown (5091)", "Stereo"),
            ),
            (
                "OrderWeight".to_string(),
                InfoRecordOut::from("Unknown (0)", "90"),
            ),
            (
                "OutputConversionType".to_string(),
                InfoRecordOut::from("Unknown (5088)", "( Lossless conversion )"),
            ),
            (
                "PanelTitle".to_string(),
                InfoRecordOut::from("AppInterfaceItemInfoTRACK", "<b>Track information</b><br>"),
            ),
            (
                "StreamFlags".to_string(),
                InfoRecordOut::from("Unknown (0)", "0"),
            ),
            (
                "TreeInfo".to_string(),
                InfoRecordOut::from("Unknown (0)", "DD Stereo English"),
            ),
            (
                "Type".to_string(),
                InfoRecordOut::from("AppTitleTreeAudio", "Audio"),
            ),
        ]);
        // ----------------------------------------------------------------
        let computed = match parse_stream_record(data) {
            Stream::Audio(x) => x,
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
            lang_code: "<unknown>".to_string(),
            lang_name: "<unknown>".to_string(),
            metadata_language_code: "eng".to_string(),
            metadata_language_name: "English".to_string(),
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

    // ====================================================================
    // subtitle stream
    // ====================================================================

    #[test]
    fn test_parse_subtitle_stream_minimal() {
        let data = HashMap::from([
            (
                "CodecId".to_string(),
                InfoRecordOut::from("Unknown (0)", "S_VOBSUB"),
            ),
            (
                "CodecLong".to_string(),
                InfoRecordOut::from("Unknown (0)", "Dvd Subtitles"),
            ),
            (
                "CodecShort".to_string(),
                InfoRecordOut::from("Unknown (0)", ""),
            ),
            (
                "MkvFlags".to_string(),
                InfoRecordOut::from("Unknown (0)", "d"),
            ),
            (
                "MkvFlagsText".to_string(),
                InfoRecordOut::from("Unknown (0)", "Default"),
            ),
            (
                "OrderWeight".to_string(),
                InfoRecordOut::from("Unknown (0)", "90"),
            ),
            (
                "OutputConversionType".to_string(),
                InfoRecordOut::from("Unknown (5088)", "( Lossless conversion )"),
            ),
            (
                "PanelTitle".to_string(),
                InfoRecordOut::from("AppInterfaceItemInfoTRACK", "<b>Track information</b><br>"),
            ),
            (
                "StreamFlags".to_string(),
                InfoRecordOut::from("Unknown (0)", "0"),
            ),
            (
                "TreeInfo".to_string(),
                InfoRecordOut::from("Unknown (0)", "English"),
            ),
            (
                "Type".to_string(),
                InfoRecordOut::from("AppTitleTreeSubPicture", "Subtitles"),
            ),
        ]);
        // ----------------------------------------------------------------
        let computed = match parse_stream_record(data) {
            Stream::Subtitle(x) => x,
            _ => panic!("Not an audio stream!"),
        };
        let expected = SubtitleStream {
            codec_id: "S_VOBSUB".to_string(),
            codec_long: "Dvd Subtitles".to_string(),
            codec_short: "".to_string(),
            lang_code: "<unknown>".to_string(),
            lang_name: "<unknown>".to_string(),
            metadata_language_code: "<unknown>".to_string(),
            metadata_language_name: "<unknown>".to_string(),
            mkv_flags: "d".to_string(),
            mkv_flags_text: "Default".to_string(),
            order_weight: 90,
            output_conversion_type: "( Lossless conversion )".to_string(),
            panel_title: "<b>Track information</b><br>".to_string(),
            stream_flags: 0,
            stream_type: "Subtitles".to_string(),
            tree_info: "English".to_string(),
        };
        // ----------------------------------------------------------------
        assert_eq!(computed, expected);
    }

    #[test]
    fn test_parse_subtitle_stream_with_lang() {
        let data = HashMap::from([
            (
                "CodecId".to_string(),
                InfoRecordOut::from("Unknown (0)", "S_VOBSUB"),
            ),
            (
                "CodecLong".to_string(),
                InfoRecordOut::from("Unknown (0)", "Dvd Subtitles"),
            ),
            (
                "CodecShort".to_string(),
                InfoRecordOut::from("Unknown (0)", ""),
            ),
            (
                "LangCode".to_string(),
                InfoRecordOut::from("Unknown (0)", "eng"),
            ),
            (
                "LangName".to_string(),
                InfoRecordOut::from("Unknown (0)", "English"),
            ),
            (
                "MkvFlags".to_string(),
                InfoRecordOut::from("Unknown (0)", "d"),
            ),
            (
                "MkvFlagsText".to_string(),
                InfoRecordOut::from("Unknown (0)", "Default"),
            ),
            (
                "OrderWeight".to_string(),
                InfoRecordOut::from("Unknown (0)", "90"),
            ),
            (
                "OutputConversionType".to_string(),
                InfoRecordOut::from("Unknown (5088)", "( Lossless conversion )"),
            ),
            (
                "PanelTitle".to_string(),
                InfoRecordOut::from("AppInterfaceItemInfoTRACK", "<b>Track information</b><br>"),
            ),
            (
                "StreamFlags".to_string(),
                InfoRecordOut::from("Unknown (0)", "0"),
            ),
            (
                "TreeInfo".to_string(),
                InfoRecordOut::from("Unknown (0)", "English"),
            ),
            (
                "Type".to_string(),
                InfoRecordOut::from("AppTitleTreeSubPicture", "Subtitles"),
            ),
        ]);
        // ----------------------------------------------------------------
        let computed = match parse_stream_record(data) {
            Stream::Subtitle(x) => x,
            _ => panic!("Not an audio stream!"),
        };
        let expected = SubtitleStream {
            codec_id: "S_VOBSUB".to_string(),
            codec_long: "Dvd Subtitles".to_string(),
            codec_short: "".to_string(),
            lang_code: "eng".to_string(),
            lang_name: "English".to_string(),
            metadata_language_code: "<unknown>".to_string(),
            metadata_language_name: "<unknown>".to_string(),
            mkv_flags: "d".to_string(),
            mkv_flags_text: "Default".to_string(),
            order_weight: 90,
            output_conversion_type: "( Lossless conversion )".to_string(),
            panel_title: "<b>Track information</b><br>".to_string(),
            stream_flags: 0,
            stream_type: "Subtitles".to_string(),
            tree_info: "English".to_string(),
        };
        // ----------------------------------------------------------------
        assert_eq!(computed, expected);
    }

    #[test]
    fn test_parse_subtitle_stream_with_meta_lang() {
        let data = HashMap::from([
            (
                "CodecId".to_string(),
                InfoRecordOut::from("Unknown (0)", "S_VOBSUB"),
            ),
            (
                "CodecLong".to_string(),
                InfoRecordOut::from("Unknown (0)", "Dvd Subtitles"),
            ),
            (
                "CodecShort".to_string(),
                InfoRecordOut::from("Unknown (0)", ""),
            ),
            (
                "MetadataLanguageCode".to_string(),
                InfoRecordOut::from("Unknown (0)", "eng"),
            ),
            (
                "MetadataLanguageName".to_string(),
                InfoRecordOut::from("Unknown (0)", "English"),
            ),
            (
                "MkvFlags".to_string(),
                InfoRecordOut::from("Unknown (0)", "d"),
            ),
            (
                "MkvFlagsText".to_string(),
                InfoRecordOut::from("Unknown (0)", "Default"),
            ),
            (
                "OrderWeight".to_string(),
                InfoRecordOut::from("Unknown (0)", "90"),
            ),
            (
                "OutputConversionType".to_string(),
                InfoRecordOut::from("Unknown (5088)", "( Lossless conversion )"),
            ),
            (
                "PanelTitle".to_string(),
                InfoRecordOut::from("AppInterfaceItemInfoTRACK", "<b>Track information</b><br>"),
            ),
            (
                "StreamFlags".to_string(),
                InfoRecordOut::from("Unknown (0)", "0"),
            ),
            (
                "TreeInfo".to_string(),
                InfoRecordOut::from("Unknown (0)", "English"),
            ),
            (
                "Type".to_string(),
                InfoRecordOut::from("AppTitleTreeSubPicture", "Subtitles"),
            ),
        ]);
        // ----------------------------------------------------------------
        let computed = match parse_stream_record(data) {
            Stream::Subtitle(x) => x,
            _ => panic!("Not an audio stream!"),
        };
        let expected = SubtitleStream {
            codec_id: "S_VOBSUB".to_string(),
            codec_long: "Dvd Subtitles".to_string(),
            codec_short: "".to_string(),
            lang_code: "<unknown>".to_string(),
            lang_name: "<unknown>".to_string(),
            metadata_language_code: "eng".to_string(),
            metadata_language_name: "English".to_string(),
            mkv_flags: "d".to_string(),
            mkv_flags_text: "Default".to_string(),
            order_weight: 90,
            output_conversion_type: "( Lossless conversion )".to_string(),
            panel_title: "<b>Track information</b><br>".to_string(),
            stream_flags: 0,
            stream_type: "Subtitles".to_string(),
            tree_info: "English".to_string(),
        };
        // ----------------------------------------------------------------
        assert_eq!(computed, expected);
    }

    // ====================================================================
    // video stream
    // ====================================================================

    #[test]
    fn test_parse_video_stream_minimal() {
        let data = HashMap::from([
            (
                "Bitrate".to_string(),
                InfoRecordOut::from("Unknown (0)", "6 Mb/s"),
            ),
            (
                "CodecId".to_string(),
                InfoRecordOut::from("Unknown (0)", "V_MPEG2"),
            ),
            (
                "CodecLong".to_string(),
                InfoRecordOut::from("Unknown (0)", "Mpeg2"),
            ),
            (
                "CodecShort".to_string(),
                InfoRecordOut::from("Unknown (0)", "Mpeg2"),
            ),
            (
                "MkvFlags".to_string(),
                InfoRecordOut::from("Unknown (0)", ""),
            ),
            (
                "OrderWeight".to_string(),
                InfoRecordOut::from("Unknown (0)", "0"),
            ),
            (
                "OutputConversionType".to_string(),
                InfoRecordOut::from("Unknown (5088)", "( Lossless conversion )"),
            ),
            (
                "PanelTitle".to_string(),
                InfoRecordOut::from("AppInterfaceItemInfoTRACK", "<b>Track information</b><br>"),
            ),
            (
                "StreamFlags".to_string(),
                InfoRecordOut::from("Unknown (0)", "0"),
            ),
            (
                "TreeInfo".to_string(),
                InfoRecordOut::from("Unknown (0)", "Mpeg2"),
            ),
            (
                "Type".to_string(),
                InfoRecordOut::from("AppTitleTreeVideo", "Video"),
            ),
            (
                "VideoAspectRatio".to_string(),
                InfoRecordOut::from("Unknown (0)", "4:3"),
            ),
            (
                "VideoFrameRate".to_string(),
                InfoRecordOut::from("Unknown (0)", "29.97 (30000/1001)"),
            ),
            (
                "VideoSize".to_string(),
                InfoRecordOut::from("Unknown (0)", "720x480"),
            ),
        ]);
        // ----------------------------------------------------------------
        let computed = match parse_stream_record(data) {
            Stream::Video(x) => x,
            _ => panic!("Not an audio stream!"),
        };
        let expected = VideoStream {
            bitrate: "6 Mb/s".to_string(),
            codec_id: "V_MPEG2".to_string(),
            codec_long: "Mpeg2".to_string(),
            codec_short: "Mpeg2".to_string(),
            metadata_language_code: "<unknown>".to_string(),
            metadata_language_name: "<unknown>".to_string(),
            mkv_flags: "".to_string(),
            order_weight: 0,
            output_conversion_type: "( Lossless conversion )".to_string(),
            panel_title: "<b>Track information</b><br>".to_string(),
            stream_flags: 0,
            stream_type: "Video".to_string(),
            tree_info: "Mpeg2".to_string(),
            video_aspect_ratio: "4:3".to_string(),
            video_framerate: "29.97 (30000/1001)".to_string(),
            video_size: "720x480".to_string(),
        };
        // ----------------------------------------------------------------
        assert_eq!(computed, expected);
    }

    #[test]
    fn test_parse_video_stream_with_meta_lang() {
        let data = HashMap::from([
            (
                "Bitrate".to_string(),
                InfoRecordOut::from("Unknown (0)", "6 Mb/s"),
            ),
            (
                "CodecId".to_string(),
                InfoRecordOut::from("Unknown (0)", "V_MPEG2"),
            ),
            (
                "CodecLong".to_string(),
                InfoRecordOut::from("Unknown (0)", "Mpeg2"),
            ),
            (
                "CodecShort".to_string(),
                InfoRecordOut::from("Unknown (0)", "Mpeg2"),
            ),
            (
                "MetadataLanguageCode".to_string(),
                InfoRecordOut::from("Unknown (0)", "eng"),
            ),
            (
                "MetadataLanguageName".to_string(),
                InfoRecordOut::from("Unknown (0)", "English"),
            ),
            (
                "MkvFlags".to_string(),
                InfoRecordOut::from("Unknown (0)", ""),
            ),
            (
                "OrderWeight".to_string(),
                InfoRecordOut::from("Unknown (0)", "0"),
            ),
            (
                "OutputConversionType".to_string(),
                InfoRecordOut::from("Unknown (5088)", "( Lossless conversion )"),
            ),
            (
                "PanelTitle".to_string(),
                InfoRecordOut::from("AppInterfaceItemInfoTRACK", "<b>Track information</b><br>"),
            ),
            (
                "StreamFlags".to_string(),
                InfoRecordOut::from("Unknown (0)", "0"),
            ),
            (
                "TreeInfo".to_string(),
                InfoRecordOut::from("Unknown (0)", "Mpeg2"),
            ),
            (
                "Type".to_string(),
                InfoRecordOut::from("AppTitleTreeVideo", "Video"),
            ),
            (
                "VideoAspectRatio".to_string(),
                InfoRecordOut::from("Unknown (0)", "4:3"),
            ),
            (
                "VideoFrameRate".to_string(),
                InfoRecordOut::from("Unknown (0)", "29.97 (30000/1001)"),
            ),
            (
                "VideoSize".to_string(),
                InfoRecordOut::from("Unknown (0)", "720x480"),
            ),
        ]);
        // ----------------------------------------------------------------
        let computed = match parse_stream_record(data) {
            Stream::Video(x) => x,
            _ => panic!("Not an audio stream!"),
        };
        let expected = VideoStream {
            bitrate: "6 Mb/s".to_string(),
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
            video_framerate: "29.97 (30000/1001)".to_string(),
            video_size: "720x480".to_string(),
        };
        // ----------------------------------------------------------------
        assert_eq!(computed, expected);
    }
}

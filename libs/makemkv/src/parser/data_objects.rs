/*
    data objects
*/

use std::collections::BTreeMap;

use crate::{
    api::DriveRecord,
    streams::{AudioStream, SubtitleStream, VideoStream},
};

// CINFO:<...>
#[derive(Clone, Debug, Default, PartialEq, serde::Serialize)]
pub struct ContentAttributes {
    pub comment: Option<String>,
    pub content_type: Option<String>,
    pub metadata_language_code: Option<String>,
    pub metadata_language_name: Option<String>,
    pub name: Option<String>,
    pub order_weight: Option<usize>,
    pub panel_title: Option<String>,
    pub tree_info: Option<String>,
    pub volume_name: Option<String>,
}

// TINFO:<...>
#[derive(Clone, Debug, Default, PartialEq, serde::Serialize)]
pub struct TitleAttributes {
    pub angle_info: Option<String>,
    pub chapter_count: usize,
    pub comment: Option<String>,
    pub disk_size: Option<String>,
    pub disk_size_bytes: usize,
    pub duration: Option<String>,
    pub metadata_language_code: Option<String>,
    pub metadata_language_name: Option<String>,
    pub name: Option<String>,
    pub order_weight: usize,
    pub original_title_id: Option<String>,
    pub output_file_name: Option<String>,
    pub panel_title: Option<String>,
    pub segments_count: usize,
    pub segments_map: Option<String>,
    pub source_file_name: Option<String>,
    pub tree_info: Option<String>,
}

// SINFO:<...>
#[derive(Clone, Debug, Default, PartialEq, serde::Serialize)]
pub struct StreamAttributes {
    pub audio_channel_layout_name: Option<String>,
    pub audio_channels_count: Option<usize>,
    pub audio_sample_rate: Option<usize>,
    pub audio_sample_size: Option<String>,
    pub bitrate: Option<String>,
    pub codec_id: Option<String>,
    pub codec_long: Option<String>,
    pub codec_short: Option<String>,
    pub language_code: Option<String>,
    pub language_name: Option<String>,
    pub metadata_language_code: Option<String>,
    pub metadata_language_name: Option<String>,
    pub mkv_flags_text: Option<String>,
    pub mkv_flags: Option<String>,
    pub name: Option<String>,
    pub offset_sequence_id: Option<String>,
    pub order_weight: Option<usize>,
    pub output_codec_short: Option<String>,
    pub output_conversion_type: Option<String>,
    pub output_format: Option<String>,
    pub output_format_description: Option<String>,
    pub panel_title: Option<String>,
    pub stream_flags: Option<usize>,
    pub stream_type: Option<String>,
    pub tree_info: Option<String>,
    pub video_aspect_ratio: Option<String>,
    pub video_frame_rate: Option<String>,
    pub video_size: Option<String>,
}

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
pub struct MakeMkvConfig {
    pub min_length: usize,
}

#[derive(Clone, Debug, PartialEq, serde::Serialize)]
pub struct MakeMkvRecord {
    pub version: String,
    pub config: MakeMkvConfig,
}

#[derive(Clone, Debug, Default, PartialEq, serde::Serialize)]
pub struct StreamsRecord {
    pub audio: BTreeMap<usize, AudioStream>,
    pub subtitles: BTreeMap<usize, SubtitleStream>,
    pub video: BTreeMap<usize, VideoStream>,
}

#[derive(Clone, Debug, Default, PartialEq, serde::Serialize)]
pub struct TitleRecord {
    pub info: TitleAttributes,
    pub streams: StreamsRecord,
}

#[derive(Clone, Debug, PartialEq, serde::Serialize)]
pub struct ContentRecord {
    pub info: ContentAttributes,
    pub titles: BTreeMap<usize, TitleRecord>,
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

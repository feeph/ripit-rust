// document expected values provided by MakeMKV
// (we might discover differences later on)
// --------------------------------------------------------------------
// always present
// --------------------------------------------------------------------
// Bitrate:
//     code: Unknown (0)
//     value: 6 Mb/s
// CodecId:
//     code: Unknown (0)
//     value: V_MPEG2
// CodecLong:
//     code: Unknown (0)
//     value: Mpeg2
// CodecShort:
//     code: Unknown (0)
//     value: Mpeg2
// MkvFlags:
//     code: Unknown (0)
//     value: ''
// OrderWeight:
//     code: Unknown (0)
//     value: '0'
// OutputConversionType:
//     code: Unknown (5088)
//     value: ( Lossless conversion )
// PanelTitle:
//     code: AppInterfaceItemInfoTRACK
//     value: <b>Track information</b><br>
// StreamFlags:
//     code: Unknown (0)
//     value: '0'
// TreeInfo:
//     code: Unknown (0)
//     value: Mpeg2
// Type:
//     code: AppTitleTreeVideo
//     value: Video
// VideoAspectRatio:
//     code: Unknown (0)
//     value: 4:3
// VideoFrameRate:
//     code: Unknown (0)
//     value: 29.97 (30000/1001)
// VideoSize:
//     code: Unknown (0)
//     value: 720x480
// --------------------------------------------------------------------
// optional
// --------------------------------------------------------------------
// MetadataLanguageCode:
//     code: Unknown (0)
//     value: eng
// MetadataLanguageName:
//     code: Unknown (0)
//     value: English

#[derive(Clone, Debug, PartialEq, serde::Serialize)]
pub struct VideoStream {
    pub bitrate: String,
    pub codec_id: String,
    pub codec_long: String,
    pub codec_short: String,
    pub metadata_language_code: String,
    pub metadata_language_name: String,
    pub mkv_flags: String,
    pub order_weight: usize,
    pub output_conversion_type: String,
    pub panel_title: String,
    pub stream_flags: usize,
    pub tree_info: String,
    pub stream_type: String,
    pub video_aspect_ratio: String,
    pub video_frame_rate: String,
    pub video_size: String,
}

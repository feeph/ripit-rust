// document expected values provided by MakeMKV
// (we might discover differences later on)
// --------------------------------------------------------------------
// always present
// --------------------------------------------------------------------
// AudioChannelLayoutName:
//     code: Unknown (0)
//     value: stereo
// AudioChannelsCount:
//     code: Unknown (0)
//     value: '2'
// AudioSampleRate:
//     code: Unknown (0)
//     value: '48000'
// Bitrate:
//     code: Unknown (0)
//     value: 224 Kb/s
// CodecId:
//     code: Unknown (0)
//     value: A_AC3
// CodecLong:
//     code: Unknown (0)
//     value: Dolby Digital
// CodecShort:
//     code: Unknown (0)
//     value: DD
// MkvFlags:
//     code: Unknown (0)
//     value: d
// MkvFlagsText:
//     code: Unknown (0)
//     value: Default
// Name:
//     code: Unknown (5091)
//     value: Stereo
// OrderWeight:
//     code: Unknown (0)
//     value: '90'
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
//     value: DD Stereo English
// Type:
//     code: AppTitleTreeAudio
//     value: Audio
// --------------------------------------------------------------------
// optional
// --------------------------------------------------------------------
// LangCode:
//     code: Unknown (0)
//     value: eng
// LangName:
//     code: Unknown (0)
//     value: English
// MetadataLanguageCode:
//     code: Unknown (0)
//     value: eng
// MetadataLanguageName:
//     code: Unknown (0)
//     value: English

#[derive(Clone, Debug, PartialEq, serde::Serialize)]
pub struct AudioStream {
    pub audio_channel_layout_name: String,
    pub audio_channels_count: usize,
    pub audio_sample_rate: usize,
    pub bitrate: String,
    pub codec_id: String,
    pub codec_long: String,
    pub codec_short: String,
    pub lang_code: String,
    pub lang_name: String,
    pub metadata_language_code: String,
    pub metadata_language_name: String,
    pub mkv_flags: String,
    pub mkv_flags_text: String,
    pub name: String,
    pub order_weight: usize,
    pub output_conversion_type: String,
    pub panel_title: String,
    pub stream_flags: usize,
    pub stream_type: String,
    pub tree_info: String,
}

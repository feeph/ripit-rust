// document expected values provided by MakeMKV
// (we might discover differences later on)
// --------------------------------------------------------------------
// always present
// --------------------------------------------------------------------
// CodecId:
//     code: Unknown (0)
//     value: S_VOBSUB
// CodecLong:
//     code: Unknown (0)
//     value: Dvd Subtitles
// CodecShort:
//     code: Unknown (0)
//     value: ''
// MkvFlags:
//     code: Unknown (0)
//     value: d
// MkvFlagsText:
//     code: Unknown (0)
//     value: Default
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
//     value: ' English'
// Type:
//     code: AppTitleTreeSubPicture
//     value: Subtitles
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
pub struct SubtitleStream {
    pub codec_id: String,
    pub codec_long: String,
    pub codec_short: String,
    pub lang_code: String,
    pub lang_name: String,
    pub metadata_language_code: String,
    pub metadata_language_name: String,
    pub mkv_flags: String,
    pub mkv_flags_text: String,
    pub order_weight: usize,
    pub output_conversion_type: String,
    pub panel_title: String,
    pub stream_flags: usize,
    pub stream_type: String,
    pub tree_info: String,
}

/*
    a hierarchical representation of the disc's content
    (uses the CINFO, TINFO & SINFO records)
*/

// standard library imports
use std::collections::BTreeMap;

// third-party imports
// <no third-party imports>

// crate-provided imports
use crate::apdefs_h::ItemAttributeId;
use crate::api::InfoRecord;
use crate::runner::streams::StreamRecord;

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

#[derive(Clone, Debug, Default, PartialEq, serde::Serialize)]
pub struct TitleRecord {
    pub info: TitleAttributes,
    pub streams: BTreeMap<usize, StreamRecord>,
}

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

impl ContentAttributes {
    pub fn new() -> Self {
        Self {
            comment: None,
            content_type: None,
            metadata_language_code: None,
            metadata_language_name: None,
            name: None,
            order_weight: None,
            panel_title: None,
            tree_info: None,
            volume_name: None,
        }
    }
}

#[derive(Clone, Debug, PartialEq, serde::Serialize)]
pub struct DiscContent {
    pub info: ContentAttributes,
    pub titles: BTreeMap<usize, TitleRecord>,
}

impl DiscContent {
    pub fn new() -> Self {
        Self {
            info: ContentAttributes::new(),
            titles: BTreeMap::new(),
        }
    }
}

pub enum UpdateError {
    InternalError,
    UnknownStreamType,
    UnknownValue,
}

impl DiscContent {
    pub fn update_disc_attribute(&mut self, ir: &InfoRecord) -> Result<(), UpdateError> {
        // get the content's info record
        let cir = &mut self.info;

        // update the value
        let value = ir.value.clone();
        match ir.attr_val {
            Some(ItemAttributeId::Comment) => {
                cir.comment = Some(value);
            }
            Some(ItemAttributeId::Name) => {
                cir.name = Some(value);
            }
            Some(ItemAttributeId::Type) => {
                cir.content_type = Some(value);
            }
            Some(ItemAttributeId::MetadataLanguageCode) => {
                cir.metadata_language_code = Some(value);
            }
            Some(ItemAttributeId::MetadataLanguageName) => {
                cir.metadata_language_name = Some(value);
            }
            Some(ItemAttributeId::OrderWeight) => {
                cir.order_weight = value.parse::<usize>().ok();
            }
            Some(ItemAttributeId::PanelTitle) => {
                cir.panel_title = Some(value);
            }
            Some(ItemAttributeId::TreeInfo) => {
                cir.tree_info = Some(value);
            }
            Some(ItemAttributeId::VolumeName) => {
                cir.volume_name = Some(value);
            }
            // error handling
            None => {
                // internal error: probably our own fault
                return Err(UpdateError::InternalError);
            }
            _ => {
                // unknown value: unexpected makemkvcon output
                return Err(UpdateError::UnknownValue);
            }
        }

        Ok(())
    }

    pub fn update_title_attribute(
        &mut self,
        tid: usize,
        ir: &InfoRecord,
    ) -> Result<(), UpdateError> {
        // get the title's info record
        let tir = self.titles.entry(tid).or_default();

        // update the value
        let value = ir.value.clone();
        match ir.attr_val {
            Some(ItemAttributeId::AngleInfo) => {
                tir.info.angle_info = Some(value);
            }
            Some(ItemAttributeId::ChapterCount) => {
                // panics if 'ChapterCount' can't be parsed as a number
                tir.info.chapter_count = value.parse::<usize>().unwrap();
            }
            Some(ItemAttributeId::Comment) => {
                tir.info.comment = Some(value);
            }
            Some(ItemAttributeId::DiskSize) => {
                tir.info.disk_size = Some(value);
            }
            Some(ItemAttributeId::DiskSizeBytes) => {
                // panics if 'DiskSizeBytes' can't be parsed as a number
                tir.info.disk_size_bytes = value.parse::<usize>().unwrap();
            }
            Some(ItemAttributeId::Duration) => {
                tir.info.duration = Some(value);
            }
            Some(ItemAttributeId::MetadataLanguageCode) => {
                tir.info.metadata_language_code = Some(value);
            }
            Some(ItemAttributeId::MetadataLanguageName) => {
                tir.info.metadata_language_name = Some(value);
            }
            Some(ItemAttributeId::Name) => {
                tir.info.name = Some(value);
            }
            Some(ItemAttributeId::OutputFileName) => {
                tir.info.output_file_name = Some(value);
            }
            Some(ItemAttributeId::OrderWeight) => {
                // panics if 'OrderWeight' can't be parsed as a number
                tir.info.order_weight = value.parse::<usize>().unwrap();
            }
            Some(ItemAttributeId::OriginalTitleId) => {
                tir.info.original_title_id = Some(value);
            }
            Some(ItemAttributeId::PanelTitle) => {
                tir.info.panel_title = Some(value);
            }
            Some(ItemAttributeId::SourceFileName) => {
                tir.info.source_file_name = Some(value);
            }
            Some(ItemAttributeId::SegmentsCount) => {
                // panics if 'SegmentsCount' can't be parsed as a number
                tir.info.segments_count = value.parse::<usize>().unwrap();
            }
            Some(ItemAttributeId::SegmentsMap) => {
                tir.info.segments_map = Some(value);
            }
            Some(ItemAttributeId::TreeInfo) => {
                tir.info.tree_info = Some(value);
            }
            // error handling
            None => {
                // internal error: probably our own fault
                return Err(UpdateError::InternalError);
            }
            _ => {
                // unknown value: unexpected makemkvcon output
                return Err(UpdateError::UnknownValue);
            }
        }

        Ok(())
    }

    pub fn insert_stream_record(
        &mut self,
        tid: usize,
        sid: usize,
        sr: &StreamRecord,
    ) -> Result<(), UpdateError> {
        let tir = self.titles.entry(tid).or_default();
        tir.streams.insert(sid, sr.clone());

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_update_disc_attributes() {
        let mut dc = DiscContent::new();
        let _ = dc.update_disc_attribute(&InfoRecord::new(1, 6206, "DVD disc"));
        let _ = dc.update_disc_attribute(&InfoRecord::new(2, 0, "Disc 1"));
        // ----------------------------------------------------------------
        let computed = dc.info;
        let expected = ContentAttributes {
            comment: None,
            content_type: Some("DVD disc".to_string()),
            metadata_language_code: None,
            metadata_language_name: None,
            name: Some("Disc 1".to_string()),
            order_weight: None,
            panel_title: None,
            tree_info: None,
            volume_name: None,
        };
        // ----------------------------------------------------------------
        assert_eq!(computed, expected);
    }
}

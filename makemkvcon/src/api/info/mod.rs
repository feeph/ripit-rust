/*
    generic info record (used by CINFO, TINFO and SINFO)
    - same as ContentInfo
    - same as TitleInfo without 'title_id'
    - same as StreamInfo without 'title_id' and 'stream_id'
*/

use crate::apdefs_h::{ItemAttributeCode, ItemAttributeId, parse_item_attribute_id, parse_item_attribute_code};

#[allow(unused_imports)]
use log::{debug, error, info, warn};

#[derive(Clone, Debug, PartialEq, serde::Serialize)]
pub struct InfoRecord {
    pub attr_num: u32,
    pub attr_val: Option<ItemAttributeId>,
    pub code_num: u32,
    pub code_val: Option<ItemAttributeCode>,
    pub value: String,
}

impl InfoRecord {
    pub fn new(attr_num: u32, code_num: u32, value: &str) -> Self {
        Self {
            attr_num,
            attr_val: parse_item_attribute_id(attr_num),
            code_num,
            code_val: parse_item_attribute_code(code_num),
            value: value.to_string(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_info_all_known() {
        // ----------------------------------------------------------------
        let computed = InfoRecord::new(1,6206, "DVD disc");
        let expected = InfoRecord{
            attr_num: 1,
            attr_val: Some(ItemAttributeId::Type),
            code_num: 6206,
            code_val: Some(ItemAttributeCode::DvdTypeDisk),
            value: "DVD disc".to_string(),
        };
        // ----------------------------------------------------------------
        assert_eq!(computed, expected);
    }

    #[test]
    fn parse_info_unknown_attribute() {
        // ----------------------------------------------------------------
        let computed = InfoRecord::new(99, 6206, "Disc 1");
        let expected = InfoRecord{
            attr_num: 99,
            attr_val: None,
            code_num: 6206,
            code_val: Some(ItemAttributeCode::DvdTypeDisk),
            value: "Disc 1".to_string(),
        };
        // ----------------------------------------------------------------
        assert_eq!(computed, expected);
    }

    #[test]
    fn parse_info_unknown_code() {
        // ----------------------------------------------------------------
        let computed = InfoRecord::new(2,0, "Disc 1");
        let expected = InfoRecord{
            attr_num: 2,
            attr_val: Some(ItemAttributeId::Name),
            code_num: 0,
            code_val: None,
            value: "Disc 1".to_string(),
        };
        // ----------------------------------------------------------------
        assert_eq!(computed, expected);
    }

}

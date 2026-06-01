/*
    parser and abstraction layer for MakeMKV's console application
    (makemkvcon)

    The console application produces a line-based output that must be
    parsed before being able to convert it into a hierarchical data
    structure.

    The TINFO, CINFO and SINFO records are especially problematic since
    semantic meaning is encoded into numerical values, e.g. the record
    `TINFO:0,11,0,"397295616"` doesn't mean anything until you know that
    the number 11 is an internal value for 'disk size in bytes'.

    The format of these records is documented on 
    <https://www.makemkv.com/developers/usage.txt> which is a good start.
    (but can't be trusted since at least TCOUNT, TINFO and SINFO are wrong)
*/

mod cinfo;
mod drive;
mod msg;
mod info;
mod sinfo;
mod tcount;
mod tinfo;

pub use cinfo::{parse_content_info_data};
pub use drive::{DriveRecord, ContentType, parse_drive_record_data};
pub use info::InfoRecord;
pub use msg::{MessageRecord, parse_msg_data};
pub use sinfo::parse_stream_info_data;
pub use tcount::parse_title_count_data;
pub use tinfo::{parse_title_info_data};

// documented usage.txt but not implemented
// ----------------------------------------

// Current and total progress title
// PRGC:code,id,name
// PRGT:code,id,name
// code - unique message code
// id - operation sub-id
// name - name string

// Progress bar values for current and total progress
// PRGV:current,total,max
// current - current progress value
// total - total progress value
// max - maximum possible value for a progress bar, constant

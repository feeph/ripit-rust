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
mod drv;
mod info;
mod line_parser;
mod msg;
mod prgc;
mod prgt;
mod prgv;
mod sinfo;
mod tcount;
mod tinfo;

pub use cinfo::parse_cinfo_data;
pub use drv::{ContentType, DrvRecord, DrvStatus, parse_drv_data};
pub use info::InfoRecord;
pub use msg::{MsgRecord, parse_msg_data};
pub use prgc::{ProgressCurrentRecord, parse_prgc_data};
pub use prgt::{ProgressTotalRecord, parse_prgt_data};
pub use prgv::{ProgressValueRecord, parse_prgv_data};
pub use sinfo::parse_sinfo_data;
pub use tcount::parse_tcount_data;
pub use tinfo::parse_tinfo_data;

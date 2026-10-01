/*!
    preserve relevant context while parsing makemkvcon's line-based output
*/

// standard library imports
use std::collections::HashMap;

// third-party imports
#[allow(unused_imports)]
use log::{debug, error, info, warn};
use tokio::sync::mpsc::Sender;

// crate-provided imports
use crate::apdefs_h::DrvStatus;
use crate::api::{
    DrvRecord, MsgRecord, ProgressCurrentRecord, ProgressTotalRecord, ProgressValueRecord,
};
use crate::parser::{ParsedOutputLine, parse_output_line};
use crate::runner::disc_content::{DiscContent, UpdateError};

// ------------------------------------------------------------------------
// public interface
// ------------------------------------------------------------------------

#[derive(Debug)]
pub enum MakeMkvEvent {
    DRV(DrvRecord),
    MSG(MsgRecord),
    TCOUNT(usize),
    CINFO(String),
    TINFO(String),
    SINFO(String),
    PRGC(ProgressCurrentRecord),
    PRGT(ProgressTotalRecord),
    PRGV(ProgressValueRecord),
}

type SacType = HashMap<usize, HashMap<usize, HashMap<u32, String>>>;

pub struct ParserContext {
    // video, audio and subtitle streams needs special attention:
    // the attributes are provided as a sequential stream but we need
    // random access to be able to identify the stream's type and return
    // the correct type -> create an intermediate stream attribute cache
    sac: SacType,
    dc: DiscContent,
    tx: Sender<MakeMkvEvent>,
    source: String,
    title_count: usize,
    prgt_last: u32,
    prgc_last: u32,
    prg_max: u32,
}

impl ParserContext {
    pub fn new(source: &str, tx: Sender<MakeMkvEvent>) -> Self {
        ParserContext {
            sac: SacType::new(),
            dc: DiscContent::new(),
            tx,
            source: source.to_string(),
            title_count: 0,
            prgt_last: 0,
            prgc_last: 0,
            prg_max: 0,
        }
    }

    pub async fn process_output(&mut self, line: &str) {
        // please note: the reported percentages for some progress values
        // are completely bonkers and may reset back to zero without any
        // obvious reason:
        // ----------------------------------------------------------------
        // PRGT:5018,0,"Scanning CD-ROM devices"
        // PRGC:5018,0,"Scanning CD-ROM devices"
        // PRGV:0,0,65536                    cur:   0% tot:   0%
        // PRGV:0,0,65536                    cur:   0% tot:   0%
        // PRGV:65536,0,65536                cur: 100% tot: 100%
        // PRGV:65536,65536,65536            cur: 100% tot: 100%
        // PRGV:0,65536,65536                cur:   0% tot: 100% <- !!!
        // PRGV:0,0,65536                    cur:   0% tot:   0% <- !!!
        // ----------------------------------------------------------------
        // the indicated two records should be ignored because:
        // - 'current' percentage resets without a PRGC record
        // - 'total' percentage resets without a PRGT record

        let parsed = parse_output_line(line.as_bytes());
        match parsed {
            // extract messages
            ParsedOutputLine::MSG(msg) => {
                let event = MakeMkvEvent::MSG(msg.to_owned());
                self.tx.send(event).await.unwrap();

                // makemkvcon may stop without reaching 100%:
                // --------------------------------------------------------
                // PRGC:3104,0,"Decrypting data"
                // PRGV:0,36086,65536
                // <…>
                // PRGV:65536,43967,65536     <-- PRGT stops at (43967) 67%
                // MSG:5011,0,0,"Operation successfully completed","Operation successfully completed"
                // --------------------------------------------------------
                // PRGC:5046,454,"Copying file"
                // PRGV:0,7377,65536
                // <…>
                // PRGV:65535,65525,65536     <-- PRGT stops at (65525) 99%
                // MSG:5070,128,0,"Backup done","Backup done"
                // MSG:5081,260,0,"Backup done.","Backup done."
                // --------------------------------------------------------
                if [5011, 5070, 5081].contains(&msg.code)
                    && (self.prgt_last < self.prg_max || self.prgc_last < self.prg_max)
                {
                    self.prgt_last = self.prg_max; // 'total'
                    self.prgc_last = self.prg_max; // 'current'
                    let prgv = ProgressValueRecord::new(
                        &self.source,
                        self.prgc_last,
                        self.prgt_last,
                        self.prg_max,
                    );
                    self.tx.send(MakeMkvEvent::PRGV(prgv)).await.unwrap();
                }
            }
            // extract drive-related data
            ParsedOutputLine::DRV(drv) => {
                debug!("Parsing drive '{}'.", line);
                // skip DRV records relating to non-existing drives
                if drv.drive_status != Some(DrvStatus::NoDrive) {
                    let event = MakeMkvEvent::DRV(drv.to_owned());
                    debug!("Parsed drive '{}'.", line);
                    self.tx.send(event).await.unwrap();
                    debug!("Sent the DRV event.");
                }
            }
            // extract content-related data
            ParsedOutputLine::TCOUNT(value) => {
                // update title count variable with actual value
                self.title_count = value;
                // and report its value to the caller
                let event = MakeMkvEvent::TCOUNT(value);
                self.tx.send(event).await.unwrap();
            }
            ParsedOutputLine::CINFO(ir) => {
                match self.dc.update_disc_attribute(&ir) {
                    Ok(_) => {}
                    Err(UpdateError::InternalError) => {
                        panic!(
                            "Failed to process CINFO record: Internal error! (id: {}, line: {})",
                            ir.attr_num, line
                        );
                    }
                    Err(UpdateError::UnknownStreamType) => {
                        // do nothing, limited to SINFO
                    }
                    Err(UpdateError::UnknownValue) => {
                        todo!(
                            "Failed to process CINFO record: Unknown attribute! (id: {}, line: {})",
                            ir.attr_val.as_ref().unwrap(),
                            line
                        );
                    }
                }
            }
            ParsedOutputLine::TINFO((tid, ir)) => {
                match self.dc.update_title_attribute(tid, &ir) {
                    Ok(_) => {}
                    Err(UpdateError::InternalError) => {
                        panic!(
                            "Failed to process TINFO record: Internal error! (id: {}, line: {})",
                            ir.attr_num, line
                        );
                    }
                    Err(UpdateError::UnknownStreamType) => {
                        // do nothing, limited to SINFO
                    }
                    Err(UpdateError::UnknownValue) => {
                        todo!(
                            "Failed to process TINFO record: Unknown attribute! (id: {}, line: {})",
                            ir.attr_val.as_ref().unwrap(),
                            line
                        );
                    }
                }
            }
            ParsedOutputLine::SINFO((tid, sid, ir)) => {
                // store this attribute in the stream attribute cache
                let tir = self.sac.entry(tid).or_default();
                let sar = tir.entry(sid).or_default();
                sar.insert(ir.attr_num, ir.value.clone());
            }
            // extract progress-related data
            ParsedOutputLine::PRGT((code, id, name)) => {
                // from <https://www.makemkv.com/developers/usage.txt>:
                // ----------------------------------------------------
                // Current and total progress title
                //
                // PRGC:code,id,name
                // PRGT:code,id,name
                // code - unique message code
                // id   - operation sub-id
                // name - name string
                // ----------------------------------------------------

                // cleanup: make sure a 100% indication is emitted for the
                // previous PRGT event before creating a new one
                //
                // multiple PRGT's are known to stop before reaching 'max',
                // e.g. "Opening DVD disc" may stop at 69% (45211)
                // --------------------------------------------------------
                // PRGT:3100,0,"Opening DVD disc"              cur:  tot:
                // <…>
                // PRGV:65536,45211,65536                      100%  >69%<
                // PRGV:0,45211,65536                            0%   69%
                // PRGV:0,0,65536                                0%    0%
                // PRGT:5024,0,"Saving all titles to MKV files"
                // --------------------------------------------------------
                if self.prgt_last < self.prg_max {
                    debug!(
                        "Previous PRGT did not complete: {} < {}.",
                        self.prgt_last, self.prg_max
                    );
                    debug!("Finalizing the incomplete PRGT record ourselves.");
                    let prgv = ProgressValueRecord::new(
                        &self.source,
                        self.prg_max, // set 'current' to 100%
                        self.prg_max, // set 'total' to 100%
                        self.prg_max,
                    );
                    self.tx.send(MakeMkvEvent::PRGV(prgv)).await.unwrap();
                }

                // process the current PRGT event
                let prgt = ProgressTotalRecord::new(&self.source, code, id, name);
                self.tx.send(MakeMkvEvent::PRGT(prgt)).await.unwrap();

                // reset 'current' and 'total' progress
                self.prgc_last = 0;
                self.prgt_last = 0;
                // zero out 'max' value to indicate the maximum is unknown
                self.prg_max = 0;
            }
            ParsedOutputLine::PRGC((code, id, name)) => {
                // from <https://www.makemkv.com/developers/usage.txt>:
                // ----------------------------------------------------
                // Current and total progress title
                //
                // PRGC:code,id,name
                // PRGT:code,id,name
                // code - unique message code
                // id - operation sub-id
                // name - name string
                // ----------------------------------------------------

                // cleanup: make sure a 100% indication is emitted for the
                // previous PRGC before creating a new one
                //
                // multiple PRGC's are known to stop before reaching 'max',
                // e.g. "Processing title sets" may stop at 89% (58637)
                // --------------------------------------------------------
                // PRGT:3100,0,"Opening DVD disc"
                // PRGC:3102,0,"Processing title sets"         cur:  tot:
                // PRGV:0,0,65536                                0%    0%
                // <…>
                // PRGV:58637,7051,65536                       >89%<  11%
                // PRGC:3120,1,"Scanning contents"
                // PRGV:0,7051,65536                             0%   11%
                // --------------------------------------------------------
                if self.prgc_last < self.prg_max {
                    let prgv = ProgressValueRecord::new(
                        &self.source,
                        self.prg_max,   // set 'current' to 100%
                        self.prgt_last, // leave 'total' unchanged
                        self.prg_max,
                    );
                    self.tx.send(MakeMkvEvent::PRGV(prgv)).await.unwrap();
                }

                // process the current PRGC event
                let prgc = ProgressCurrentRecord::new(&self.source, code, id, name);
                self.tx.send(MakeMkvEvent::PRGC(prgc)).await.unwrap();

                // reset 'current' progress
                // (preserve recorded 'total' and 'max' value)
                self.prgc_last = 0;
            }
            ParsedOutputLine::PRGV((current, total, maximum)) => {
                // from <https://www.makemkv.com/developers/usage.txt>:
                // ----------------------------------------------------
                // Progress bar values for current and total progress
                //
                // PRGV:current,total,max
                // current - current progress value
                // total   - total progress value
                // max     - maximum possible value for a progress bar
                // ----------------------------------------------------

                // process PRGV event
                //
                // a) it is possible for 'current' to remain unchanged
                //    while 'total' increases:
                // --------------------------------------------------------
                // PRGV:65505,64683,65536
                // PRGV:65505,65270,65536
                //        ^-- 'current' remained at previous value
                // --------------------------------------------------------
                // b) it is possible for 'current' to increase while
                //    while 'total' remains unchanged
                // --------------------------------------------------------
                // PRGV:0,0,65536
                // PRGV:65536,0,65536
                //            ^-- 'total' remained at previous value
                // --------------------------------------------------------
                //
                // "Scanning CD-ROM devices" is known to generate really
                // weird output:
                //   - 'current' resets from 65536 to 0 without a PRGC
                //   - 'total' resets from 65536 to 0 without a PRGT
                // --------------------------------------------------------
                // PRGT:5018,0,"Scanning CD-ROM devices"
                // PRGC:5018,0,"Scanning CD-ROM devices"
                // PRGV:0,0,65536
                // PRGV:0,0,65536
                // PRGV:65536,0,65536
                // PRGV:65536,65536,65536
                // PRGV:0,65536,65536             <-- 'current' resets to 0
                // PRGV:0,0,65536                 <-- 'total' resets to 0
                // PRGT:3100,0,"Opening DVD disc"
                // --------------------------------------------------------
                debug!(
                    "current: {:5} -> {:5} || total: {:5} -> {:5} || max: {:5} -> {:5}",
                    self.prgc_last, current, self.prgt_last, total, self.prg_max, maximum
                );
                if current >= self.prgc_last && total >= self.prgt_last {
                    let prgv = ProgressValueRecord::new(&self.source, current, total, maximum);
                    self.tx.send(MakeMkvEvent::PRGV(prgv)).await.unwrap();

                    // remember updated progress values
                    self.prgc_last = current;
                    self.prgt_last = total;
                    self.prg_max = maximum;
                } else if total < self.prgt_last {
                    debug!(
                        "Ignoring '{}' because 'total' progress resets without a PRGT. (old: {}, new: {})",
                        line, self.prgt_last, total
                    );
                } else if current < self.prgc_last {
                    debug!(
                        "Ignoring '{}' because 'current' progress resets without a PRGC. (old: {}, new: {})",
                        line, self.prgc_last, current
                    );
                } else {
                    warn!(
                        "huh?! current: {:5} -> {:5} || total: {:5} -> {:5} || max: {:5} -> {:5}",
                        self.prgc_last, current, self.prgt_last, total, self.prg_max, maximum
                    );
                }
            }
        }
    }

    pub fn get_dc(&self) -> DiscContent {
        self.dc.clone()
    }

    pub fn get_sac(&self) -> SacType {
        self.sac.clone()
    }

    pub fn get_title_count(&self) -> usize {
        self.title_count
    }
}

impl Drop for ParserContext {
    fn drop(&mut self) {
        if self.prgc_last < self.prg_max {
            warn!("prgc_last < prg_max: {} < {}", self.prgc_last, self.prg_max);
        }
        if self.prgt_last < self.prg_max {
            warn!("prgt_last < prg_max: {} < {}", self.prgt_last, self.prg_max);
        }
    }
}

// ------------------------------------------------------------------------
// private helper functions
// ------------------------------------------------------------------------

// ------------------------------------------------------------------------
// tests
// ------------------------------------------------------------------------

mod tests {

    #[allow(unused_imports)]
    use super::*;

    use std::fs::File;
    use std::io::{BufRead, BufReader};

    // backup failed, file prematurely stops at:
    // <…>
    // PRGV:65130,65156,65536
    // (without final 'MSG:5080' or 'MSG:5081')
    #[tokio::test]
    async fn test_process_output_fail1() {
        let br = read_file("src/runner/parser_context/data/blu-ray/A Few Good Men.log");
        // ----------------------------------------------------------------
        let computed = get_pvrs(br).await.pop().unwrap();
        let expected = ProgressValueRecord::new("UnitTest", 65130, 65156, 65536);
        // ----------------------------------------------------------------
        assert_eq!(computed, expected);
    }

    // backup failed, file prematurely stops at:
    // <…>
    // PRGV:65286,65282,65536
    // (without final 'MSG:5080' or 'MSG:5081')
    #[tokio::test]
    async fn test_process_output_fail2() {
        let br = read_file("src/runner/parser_context/data/dvd/Otaku no Video.log");
        // ----------------------------------------------------------------
        let computed = get_pvrs(br).await.pop().unwrap();
        let expected = ProgressValueRecord::new("UnitTest", 65286, 65282, 65536);
        // ----------------------------------------------------------------
        assert_eq!(computed, expected);
    }

    // backup succeeded, file stops at:
    // <…>
    // PRGV:65535,65533,65536
    // MSG:5070,128,0,"Backup done","Backup done"
    // MSG:5081,260,0,"Backup done.","Backup done."
    #[tokio::test]
    async fn test_process_output_ok1() {
        let br = read_file("src/runner/parser_context/data/dvd/Fish Police.log");
        // ----------------------------------------------------------------
        // ParserContext is expected to finalize the progress values
        let computed = get_pvrs(br).await.pop().unwrap();
        let expected = ProgressValueRecord::new("UnitTest", 65536, 65536, 65536);
        // ----------------------------------------------------------------
        assert_eq!(computed, expected);
    }

    // --------------------------------------------------------------------

    fn read_file(filename: &str) -> std::io::BufReader<std::fs::File> {
        let file = std::fs::File::open(filename).expect("data file must exist");
        std::io::BufReader::new(file)
    }

    async fn get_pvrs(br: BufReader<File>) -> Vec<ProgressValueRecord> {
        let (tx, mut rx) = tokio::sync::mpsc::channel::<MakeMkvEvent>(256);
        let mut pc = ParserContext::new("UnitTest", tx);
        let mut pvrs = Vec::new();
        for line in br.lines() {
            let line = line.expect("read line from fixture");
            pc.process_output(&line).await;

            while let Ok(event) = rx.try_recv() {
                if let MakeMkvEvent::PRGV(pvr) = event {
                    pvrs.push(pvr);
                }
            }
        }
        return pvrs;
    }
}

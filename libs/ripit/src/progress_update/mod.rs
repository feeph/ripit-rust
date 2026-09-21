/*!
    progress update

    This module provides an interface that is more suitable to programming.
    The core issue with the way makemkvcon provides its progress updates is
    that they are line-based and you need the context of previous lines to
    be able to understand what the PRGV record relates to:

    ```TEXT
    PRGT:5018,0,"Scanning CD-ROM devices"
    PRGC:5018,0,"Scanning CD-ROM devices"
    PRGV:0,0,65536
    PRGV:0,0,65536
    PRGV:16384,0,65536
    PRGV:16384,16384,65536
    PRGV:65536,16384,65536
    PRGV:65536,65536,65536
    PRGV:0,65536,65536
    PRGV:0,0,65536
    PRGT:5047,0,"Copying all files"
    PRGC:3102,0,"Processing title sets"
    PRGV:0,0,65536
    <...>
    ```

    The following changes are applied:
    - PRGT, PRGC and PRGV are combined into a single record, removing the
      need to keep track of the current PRGT and PRGC.
    - the range 0-65536 is provided as a percentage (0-100)
    - 'source' and 'target' are provided as part of the update to allow the
      use of a single event queue even if multiple jobs are run in parallel

    significance of "PRGT" and "PRGC" records:
    ------------------------------------------

    "PRGT" can be considered the current stage while "PRGC" is a task
    within that stage.

    When running `makemkvcon backup` the relevant stages and tasks are:

    ```TEXT
    PRGT:5018,0,"Scanning CD-ROM devices"
      PRGC:5018,0,"Scanning CD-ROM devices"

    PRGT:5047,0,"Copying all files"
      PRGC:3102,0,"Processing title sets"
      PRGC:3120,1,"Scanning contents"
      PRGC:5046,0,"Copying file"
    ```

    For a DVD or HD-DVD there is exactly one "PRGC:5046". For a Blu-Ray
    there are hundreds. This is because DVDs and HD-DVDs are extracted as
    a single ISO files, while Blu-Rays they are extracted to a directory.

    significance of "PRGV" records:
    -------------------------------

    The PRGV records provides the progress of PRGT and PRGV. The third
    value provides the maximum (always 65536) while the first and the
    second provide the actual progress value.

    The information in this record is meaningless if you don't know what
    the current PRGT and PRGC are.

*/

// standard library imports
// <none>

// third-party imports
#[allow(unused_imports)]
use log::{debug, error, info, warn};

// crate-provided imports
use crate::drives::OpticalDisc;

// ------------------------------------------------------------------------
// public interface
// ------------------------------------------------------------------------

#[derive(Clone, Debug)]
pub struct ProgressValue {
    pub code: u32,
    pub name: String,
    pub percentage: f32,
}

impl ProgressValue {
    pub fn new(code: u32, name: &str) -> Self {
        ProgressValue {
            code,
            name: name.to_string(),
            percentage: 0.0,
        }
    }
}

#[derive(Clone, Debug)]
pub struct ProgressUpdate {
    pub source: String,
    pub target: String,
    pub disc: OpticalDisc,
    /// PRGT - "progress total"
    pub prgt: ProgressValue,
    /// PRGC - "progress current"
    pub prgc: ProgressValue,
}

impl ProgressUpdate {
    // PRGT and PRGC always come before PRGV
    // if at some point "<unknown>" shows up then this indicates makemkvcon
    // is doing something weird and unexpected -> check makemkvcon's output
    pub fn new(source: &str, target: &str, disc: &OpticalDisc) -> Self {
        ProgressUpdate {
            source: source.to_owned(),
            target: target.to_owned(),
            disc: disc.clone(),
            prgt: ProgressValue::new(0, "<unknown>"),
            prgc: ProgressValue::new(0, "<unknown>"),
        }
    }

    /// e.g.: '/dev/sr0_5047'
    pub fn get_device_id(&self) -> String {
        self.source.clone()
    }

    /// e.g.: '/dev/sr0_5047'
    pub fn get_stage_id(&self) -> String {
        format!("{}_{}", self.source, self.prgt.code)
    }

    /// e.g.: "Copying all files"
    pub fn get_stage_name(&self) -> String {
        self.prgt.name.to_owned()
    }

    /// e.g.: "Copying file"
    pub fn get_task_name(&self) -> String {
        self.prgc.name.to_owned()
    }
}

/*
*/

// standard library imports
use std::collections::HashMap;

// third-party imports
#[allow(unused_imports)]
use log::{debug, error, info, warn};

// crate-provided imports
// <none>

// ------------------------------------------------------------------------
// public interface
// ------------------------------------------------------------------------

#[derive(Clone)]
pub enum Severity {
    Info,
    Warn,
    Fail,
}

pub fn default_severity_map() -> HashMap<u32, Severity> {
    // MSG:1005 - MakeMKV v1.18.3 win(x64-release) started
    // MSG:1011 - Using LibreDrive mode (v06.3 id=0FA242DD4D0B)
    // MSG:2003 various types of read errors
    // MSG:5075 - The new version 1.18.3 is available for download at http://www.makemkv.com/download/
    // MSG:3324 - Processing BD+ code using generic SVQ from builtin/generic.svq
    // MSG:3328 - BD+ code processed, got 1 FUT(s) for 20 clip(s)
    // MSG:3338 - Downloading latest SDF to <…>/.MakeMKV ...
    // MSG:3007 - Using direct disc access mode
    // MSG:3028 - Title #2 was added (7 cell(s), 0:06:07)
    // MSG:3025 - Title #3 has length of 20 seconds which is less than minimum title length of 120 seconds and was therefore skipped
    // MSG:3038 - Cells 3-7 were removed from title end
    // MSG:3006 - Opening files on harddrive at file://<…>
    // MSG:3307 - File 00006.mpls was added as title #0
    // MSG:3308 - File 00800.mpls (angle 1) was added as title #8
    // MSG:3309 - Title 00021.mpls(1) is equal to title 00006.mpls and was skipped
    // MSG:3041 - Failed to add angle #2 for title #850
    // MSG:3026 - Title #11 declared length is 0:00:00 while its real length is 0:00:16 - assuming fake title
    // MSG:3344 - Using Java runtime from <…>/bin/java
    // MSG:5014 - Saving 1 titles into directory file://<…>
    // MSG:5072 - Backing up disc into folder file://<…>
    // MSG:5085 - Loaded content hash table, will verify integrity of M2TS files.
    // <...>
    // MSG:3027 - Title #44 in VTS 10 is equal to title #37 and was skipped
    // MSG:3029 - Audio stream #4 is identical to stream #2 and was skipped
    // MSG:3030 - Subtitle stream #11 is identical to stream #9 and was skipped
    // MSG:3037 - Cells 1-1 were removed from title start
    // <...>
    // == generic ==
    // == makemkvcon backup ==
    // -- success --
    // MSG:5070 - Backup done
    // MSG:5081 - Backup done.
    // -- failure #1 --
    // MSG:5010 - Failed to open disc
    // -- failure #2 --
    // MSG:5069 - Backup failed
    // MSG:5080 - Backup failed.
    // == makemkvcon mkv ==
    // -- success --
    // MSG:5005 - 1 titles saved
    // MSG:5011 - Operation successfully completed
    // MSG:5036 - Copy complete. 1 titles saved.
    // -- filesystem or permission issue --
    // MSG:2018 - Error 'Internal error - Operation result is incorrect (178)' occurred while writing data to '<…>/C1_t04.mkv' at offset '1811939328'
    // MSG:2019 - Error 'OS error - The system cannot find the path specified' occurred while creating '<…>/B1_t00.mkv'
    // MSG:5038 - The total size of all output files may reach as much as <…> megabytes while there are only <…> megabytes free on the destination drive. Do you still want to continue?
    // -- content issues --
    // MSG:3034 - Audio stream #5 in title #7 looks empty and was skipped
    // MSG:4004 - The source file '/VIDEO_TS/VTS_01_1.VOB' is corrupt or invalid at offset 28672, attempting to work around
    // -- conflict --
    // MSG:5001 - File <…>/B1_t00.mkv already exist. Do you want to overwrite it?
    // MSG:5005 - 1 titles saved
    // -- failure --
    // MSG:5003 - Failed to save title 0 to file <…>/B1_t00.mkv
    // MSG:5004 - 0 titles saved, 1 failed
    // MSG:5037 - Copy complete. 0 titles saved, 1 failed.

    HashMap::from([
        (1005, Severity::Info),
        (1011, Severity::Info),
        (2003, Severity::Fail), // read errors
        (2018, Severity::Fail), // write error (filesystem full?)
        (2019, Severity::Fail), // cannot find the path specified
        (3006, Severity::Info),
        (3007, Severity::Info),
        (3025, Severity::Info),
        (3026, Severity::Info),
        (3027, Severity::Info), // Title is equal to another title and was skipped
        (3028, Severity::Info),
        (3029, Severity::Info), // Audio stream is identical to another stream and was skipped
        (3030, Severity::Info), // Subtitle stream is identical to another stream and was skipped
        (3034, Severity::Info), // Audio stream looks empty and was skipped
        (3037, Severity::Info), // Cells were removed from title start
        (3038, Severity::Info),
        (3041, Severity::Warn), // Failed to add angle
        (3307, Severity::Info),
        (3308, Severity::Info),
        (3309, Severity::Info),
        (3324, Severity::Info),
        (3328, Severity::Info),
        (3338, Severity::Info),
        (3344, Severity::Info),
        (4004, Severity::Warn), // The source file is corrupt or invalid, attempting to work around
        (5001, Severity::Warn), // File already exist
        (5003, Severity::Fail), // Failed to save title to file
        (5004, Severity::Fail), // 0 titles saved, 1 failed
        (5005, Severity::Info),
        (5010, Severity::Fail), // Failed to open disc
        (5011, Severity::Info),
        (5014, Severity::Info),
        (5036, Severity::Info), // "Copy complete. 1 titles saved.""
        (5037, Severity::Fail), // "Copy complete. 0 titles saved, 1 failed.""
        (5038, Severity::Warn), // Total filesize exceeds available space
        (5069, Severity::Fail), // "Backup failed" (no trailing dot)
        (5070, Severity::Info), // "Backup done" (no trailing dot)
        (5072, Severity::Info),
        (5075, Severity::Warn), // New version available
        (5080, Severity::Fail), // "Backup failed."
        (5081, Severity::Info), // "Backup done."
        (5085, Severity::Info),
    ])
}

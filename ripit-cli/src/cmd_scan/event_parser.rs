/*!
    ripit-cli scan [OPTIONS]

    (use `ripit-cli scan --help` for full usage instructions)

    usage:

    ```TEXT
    # read from an optical drive and show disc's content
    ripit-cli scan -d E:
    ripit-cli scan -d /dev/sr0

    # read from a disc image
    ripit-cli scan -i feature.iso

    # write disc's content to YAML file
    ripit-cli scan -d E:        -f YAML -o content.yaml
    ripit-cli scan -i image.iso -f YAML -o content.yaml
    ```

    output example:

    ```TEXT
    $ ripit-cli scan -d /dev/sr0
    <...>
    ```
*/

// standard library imports
use std::collections::HashMap;

// third-party imports
use dialoguer::console::{Alignment, pad_str};
#[allow(unused_imports)]
use log::{debug, error, info, trace, warn};
// crate-provided imports
use crate::progress_tracker::{ProgressTracker, Stage};
use ripit::ScanEvent;

// ------------------------------------------------------------------------
// public interface
// ------------------------------------------------------------------------

pub struct EventParser {
    pub jobs_done: usize,
    pub jobs_failed: usize,
    stages: HashMap<String, HashMap<String, Stage>>,
    msg_4004: usize,
}

impl EventParser {
    pub fn new() -> Self {
        EventParser {
            jobs_done: 0,
            jobs_failed: 0,
            msg_4004: 0,
            stages: HashMap::new(),
        }
    }

    pub fn get_msg_4004(&self) -> usize {
        self.msg_4004
    }

    pub fn parse_events(&mut self, events: &mut Vec<ScanEvent>, pt: &mut ProgressTracker) {
        // during an extraction we expect to see the following events:
        // ----------------------------------------------------------------
        // PRGT:5018,0,"Scanning CD-ROM devices"
        //   PRGC:5018,0,"Scanning CD-ROM devices"
        // PRGT:3100,0,"Opening DVD disc"
        //   PRGC:3102,0,"Processing title sets"
        //   PRGC:3120,1,"Scanning contents"
        //   PRGC:3103,0,"Processing titles"
        //   PRGC:3104,0,"Decrypting data"
        // PRGT:5024,0,"Saving all titles to MKV files"
        //   <repeating for each output file>
        //   PRGC:5057,0,"Analyzing seamless segments"
        //   PRGC:5017,0,"Saving to MKV file"
        //   PRGC:5057,1,"Analyzing seamless segments"
        //   PRGC:5017,1,"Saving to MKV file"
        //             ^-- index of generated output file
        //   </repeating for each output file>
        // ----------------------------------------------------------------
        // local convention:
        // - let's refer to PRGT records as 'stages'
        // - let's refer to PRGC records as 'tasks'
        for event in events.drain(..) {
            match event {
                ScanEvent::MsgInfo(msg) => {
                    info!(
                        "[{}] MSG:{} - {}",
                        msg.source, msg.message.code, msg.message.message
                    );
                    // pt.send_text_message(&format!("[makemkv] I: MSG:{} - {}", msg.message.code, msg.message.message));
                }
                ScanEvent::MsgWarn(msg) => {
                    if msg.message.code == 4004 {
                        self.msg_4004 += 1;
                    } else {
                        warn!(
                            "[{}] MSG:{} - {}",
                            msg.source, msg.message.code, msg.message.message
                        );
                    }
                }
                ScanEvent::MsgFail(msg) => {
                    error!(
                        "[{}] MSG:{} - {}",
                        msg.source, msg.message.code, msg.message.message
                    );
                }
                ScanEvent::ProgressT(pu) => {
                    // there should be exactly 3 stages per extraction:
                    // "Scanning…" / "Opening…" / "Saving…"
                    debug!(
                        "Create PRGT progress bar for '{}' ({}).",
                        pu.source, pu.prgt.code
                    );

                    // create a progress bar for the current stage
                    let pb_prgt_id = format!("{}_prgt", pu.source);
                    let stage_name = pu.get_stage_name();
                    let disc_name = pu.disc.get_name();
                    pt.create_progress_bar(&pb_prgt_id, disc_name, &stage_name);
                }
                ScanEvent::ProgressC(pu) => {
                    // each stage has one or more tasks
                    debug!(
                        "Create PRGC progress bar for '{}' ({}:{}).",
                        pu.source, pu.prgt.code, pu.prgc.code
                    );

                    // create a progress bar for the current task
                    // (the progress bar is anchored to its stage)
                    let pb_prgt_id = format!("{}_prgt", pu.source);
                    let pb_prgc_id = format!("{}_prgc", pu.source);
                    let device_name = pu.source.clone();
                    let task_name = pu.get_task_name();
                    pt.create_progress_bar_after(
                        &pb_prgc_id,
                        &device_name,
                        &task_name,
                        &pb_prgt_id,
                    )
                    .unwrap();
                }
                ScanEvent::ProgressValue(pu) => {
                    // "Processing title sets"
                    // "Scanning contents"
                    let device_name = pu.source.clone();
                    let stage_name = pu.get_stage_name(); // reported as 'total'
                    let task_name = pu.get_task_name(); // reported as 'current'
                    let device_stage = self
                        .stages
                        .entry(device_name.clone())
                        .or_insert(HashMap::from([(stage_name.clone(), Stage::new())]));
                    let stage = device_stage
                        .entry(stage_name.clone())
                        .or_insert(Stage::new());

                    let disc_name = pu.disc.get_name();

                    let prgt_old = stage.get_prgt();
                    let prgt_new = pu.prgt.percentage;
                    let prgc_old = stage.get_prgc();
                    let prgc_new = pu.prgc.percentage;

                    // update stored values
                    // --------------------

                    stage.update_progress(prgt_new, prgc_new);

                    // report to user
                    // --------------

                    // create a fixed-length string suitable for generating
                    // vertically aligned output
                    // (pad or truncate the disc's name as needed)
                    let disc_name_fl = pad_str(disc_name, 20, Alignment::Left, Some("…"));

                    // throttle log output: notify only if stage or
                    // percentage has changed more than 5%
                    // (prevent log-flooding)
                    if prgc_old.is_nan() || prgt_new > prgt_old || prgc_new >= prgc_old + 5.0 {
                        info!(
                            "[{}] {}: {:3.0}% (done: {}, failed: {})",
                            device_name, stage_name, prgt_new, self.jobs_done, self.jobs_failed
                        );
                    }

                    let pb_prgt_id = format!("{}_prgt", pu.source);
                    let pb_prgc_id = format!("{}_prgc", pu.source);

                    // throttle progress bars: notify only if relevant
                    // progress was made
                    // (indicatif handles fine-grained throttling)
                    //
                    // It is possible for the 'total percentage' percentage
                    // to change while the 'current percentage' (PRGC)
                    // remains at the previous value. This doesn't really
                    // make sense but it's the way it is and must be
                    // handled appropriately.
                    if prgc_old.is_nan() || prgt_new > prgt_old || prgc_new >= prgc_old + 0.1 {
                        // update progress bars for current stage and task
                        pt.update_progress_bar(
                            &pb_prgt_id,
                            &disc_name_fl,
                            &stage_name,
                            pu.prgt.percentage,
                        )
                        .unwrap();

                        // modify the task name to make it more
                        // obvious how stage and task are related:
                        // ----------------------------------------
                        // ⠸ [/dev/sr0] Copying all files (6%)    [█░░░░░░░░░░░░░░░░░░░] 00:03:25
                        // ⠹ [/dev/sr0] `--> Copying file (25%)   [████░░░░░░░░░░░░░░░░] 00:00:12
                        // ----------------------------------------
                        let task_name_mod = format!("`--> {}", task_name);
                        pt.update_progress_bar(
                            &pb_prgc_id,
                            &disc_name_fl,
                            &task_name_mod,
                            pu.prgc.percentage,
                        )
                        .unwrap();
                    } else {
                        debug!(
                            "task pct: old {:5.2}% new {:5.2}% (skip)",
                            prgc_old, prgc_new
                        );
                    }

                    if prgt_new == 100.0 {
                        // record completion
                        pt.send_text_message(&format!(
                            "🗸 [{}] '{}' finished after {} seconds.",
                            disc_name_fl,
                            stage_name,
                            stage.get_elapsed()
                        ));
                        // remove progress bars
                        pt.clear_progress_bar(&pb_prgc_id).unwrap();
                        pt.clear_progress_bar(&pb_prgt_id).unwrap();
                    }
                }
            }
        }
    }
}

// ------------------------------------------------------------------------
// private helper functions
// ------------------------------------------------------------------------

// <none>

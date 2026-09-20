/*!
    ripit-cli unshackle [OPTIONS]

    (use `ripit-cli unshackle --help` for full usage instructions)

    usage:

    ```TEXT
    # use drives "D:" and "E:":
    ripit-cli unshackle -d D: -d E: -t C:\dumped

    # use all available drives:
    ripit-cli unshackle -t C:\dumped

    # use all available drives and current directory:
    ripit-cli unshackle
    ```

    output example:

    ```TEXT
    $ ripit-cli unshackle -O -d /dev/sr1 /dev/sr2 -t /media/image
    I: Detecting available drives.                                                                                                                                                                         Found 2 drives.                                                                                                                                                                                     [/dev/sr2] Starting backup of DVD 'DVDVolume'.                                                                                                                                                      [/dev/sr1] Starting backup of DVD 'OTAKU_NO_VIDEO'.
    ⠹ [disc:0] Copying all files (11%)                  [██░░░░░░░░░░░░░░░░░░] 00:04:13
    ⠸ [disc:0] `--> Copying file (11%)                  [██░░░░░░░░░░░░░░░░░░] 00:03:15
    ⠧ [disc:1] Copying all files (5%)                   [░░░░░░░░░░░░░░░░░░░░] 00:04:09
    ⠼ [disc:1] `--> Copying file (5%)                   [░░░░░░░░░░░░░░░░░░░░] 00:03:14
    ```

    A logfile with makemkvcon's output is added to the target directory.

*/

// standard library imports
use std::collections::HashMap;
use std::path::PathBuf;

// third-party imports
use clap::{Parser, ValueHint};
#[allow(unused_imports)]
use log::{debug, error, info, warn};
use tokio::spawn;
use tokio::sync::mpsc;
use tokio::time::{Duration, sleep};

// crate-provided imports
use crate::progress_tracker::ProgressTracker;
use makemkv::ScanMode;
use ripit::{UnshackleEvent, find_drives, find_matching_drives, unshackle_disc};

#[derive(Debug, Parser)]
pub struct CmdArgs {
    // Please note:
    // Trailing dots in ///-comments are stripped by clap.
    /// Select a drive, e.g. 'D:' or '/dev/sr0' (repeatable)
    /// (implies usage of all drives if none are selected)
    #[arg(short = 'd', long = "drive", num_args = 1..)]
    drives_want: Vec<PathBuf>,

    /// Target directory
    #[arg(short = 't', long = "target-dir", default_value = ".", value_hint = ValueHint::DirPath)]
    target: std::path::PathBuf,

    /// Eject medium from the drive after backup
    #[arg(short = 'e', long = "eject-when-done")]
    eject_when_done: bool,

    /// Enable continuous mode (does not exit when done, implies eject)
    #[arg(short = 'c', long = "continuous")]
    continuous: bool,

    /// Allow overwriting existing destination files
    #[arg(short = 'O', long = "allow-overwrite")]
    allow_overwrite: bool,

    /// Enable verbose logging
    #[arg(short, long)]
    verbose: bool,

    #[clap(flatten)]
    global_opts: crate::GlobalOpts,
}

pub async fn run(args: CmdArgs, mm: &makemkv::MakeMkv) -> i32 {
    // When working with an optical drive we're 100% I/O bound. A DVD is
    // read at ~10MiB/s. A Blu-Ray allows for slightly faster reads at
    // ~20MiB/s. This is the bottle neck. Everything else is kind of
    // irrelevant since it's very likely to be fast enough.
    //
    // The only way to reduce runtime is to use multiple optical drives and
    // run multiple extractions in parallel. Up to 6 drives are going to
    // be fine on any reasonably modern hardware.
    //
    // goal: run multiple extractions in parallel
    // design:
    // - using a dedicated process for each instance of 'makemkvcon'
    //   (mode: concurrent and parallel)
    // - using threads for the internal logic while makemkvcon is running
    //   in the background
    //   (mode: concurrent, not parallel)
    //
    // CPU usage:
    // - per 'makemkvcon backup' process: half of a CPU core (Celeron N3450)
    // - for 'ripit-cli': less than one percent

    crate::logging::init_logger(args.global_opts.log_level);

    let mut pt = ProgressTracker::new();

    pt.send_text_message("I: Detecting available drives.");

    let scan_mode = ScanMode::DriveAndDisc;
    let drives_have = find_drives(mm.to_owned(), scan_mode)
        .await
        .expect("Reading drives should never fail.");
    if drives_have.is_empty() {
        pt.send_text_message("E: Found no available drives!");
        return exitcode::IOERR;
    } else {
        // convert 'Vec<OpticalDrive>' into a stringified list of sorted
        // device names, e.g. '/dev/sr0 /dev/sr1'
        let mut device_names = drives_have
            .clone()
            .into_iter()
            .map(|drv| drv.device.to_string_lossy().to_string())
            .collect::<Vec<_>>();
        device_names.sort();
        let device_names_str = device_names.join(" ");
        pt.send_text_message(&format!(
            "I: Found {} drives: {}",
            drives_have.len(),
            device_names_str
        ));
    }

    let drives_want = args.drives_want;

    let mut drives = find_matching_drives(&drives_have, &drives_want);
    debug!("drives_have: {:#?}", drives_have);
    debug!("drives_want: {:#?}", drives_want);
    debug!("filtered:    {:#?}", drives);

    let target = args.target.to_path_buf();
    info!("Using target directory: {}", target.to_string_lossy());

    let (tx, mut rx) = mpsc::channel::<UnshackleEvent>(256);

    // Initialize progress renderer (detect if stdout is a TTY)
    // TODO figure out if current shell is an interactive shell
    // - if printing to terminal    -> use progress bars
    // - if writing to file or pipe -> use line-based output
    // let is_interactive = atty::is(atty::Stream::Stdout);

    // create worker tasks for all drives with disc
    let allow_overwrite = args.allow_overwrite;
    let eject_when_done = args.eject_when_done;
    let mut workers = HashMap::new();
    for drive in drives.extract_if(.., |x| x.has_disc()) {
        info!(
            "[{}] Creating worker task for `makemkvcon backup`.",
            drive.device.to_string_lossy()
        );
        debug!("drive: {:#?}.", drive);
        let disc = drive.disc.clone().unwrap();
        let disc_name = disc.get_name();
        let disc_type = disc.get_type();
        pt.send_text_message(&format!(
            "[{}] Starting backup of {} '{}'.",
            drive.device.to_string_lossy(),
            disc_type,
            disc_name
        ));
        let mm_mkv = mm.clone();
        let source_mkv = drive.clone();
        let target_mkv = target.clone();
        let tx_tmp = tx.clone();
        let th_mkv = spawn(async move {
            unshackle_disc(
                mm_mkv,
                source_mkv,
                target_mkv,
                allow_overwrite,
                eject_when_done,
                tx_tmp,
            )
            .await
        });

        workers.insert(drive, th_mkv);
    }
    // after this loop has run:
    // - 'workers' contains a worker thread for each loaded drive
    // - 'drives' contains exclusively empty drives

    // monitor progress of worker tasks and terminate
    // parse incoming events until the spawned task finishes
    let mut stages: HashMap<String, HashMap<String, f32>> = HashMap::new();
    let mut events = Vec::new();
    let batch_size = 32;
    let mut jobs_done = 0;
    let mut jobs_failed = 0;
    loop {
        // TODO  test if new workers need to be spawned

        tokio::select! {
            // batched processing of generated events to reduce overhead
            //
            // It is important to create/clear the progress bars and avoid
            // reusing the same objects because the presented runtime value
            // is derived from the progress bar's creation time:
            //
            // ⠦ [/dev/sr1] Copying all files (0%)  [░░░░░░░░░░░░░░░░░░░░] 00:00:25
            //                                                             ^^^^^^^^
            _ = rx.recv_many(&mut events, batch_size) => {
                // parse generated events
                for event in events.drain(..) {
                    match event {
                        UnshackleEvent::MsgInfo(msg) => {
                            let device = msg.device.to_string_lossy();
                            debug!("[{}] MSG:{} - {}", device, msg.message.code, msg.message.message);
                            info!("[{}] [makemkvcon] {}", device, msg.message.message);
                        },
                        UnshackleEvent::MsgWarn(msg) => {
                            let device = msg.device.to_string_lossy();
                            debug!("[{}] MSG:{} - {}", device, msg.message.code, msg.message.message);
                            warn!("[{}] [makemkvcon] {}", device, msg.message.message);
                        },
                        UnshackleEvent::MsgFail(msg) => {
                            let device = msg.device.to_string_lossy();
                            debug!("[{}] MSG:{} - {}", device, msg.message.code, msg.message.message);
                            error!("[{}] [makemkvcon] {}", device, msg.message.message);
                        },
                        UnshackleEvent::ProgressT(pu) => {
                            // "Scanning CD-ROM devices" / "Copying all files"
                            // There should be exactly two events per backup
                            // job: scanning and copying.

                            // create progress bar for current stage
                            let pb_prgt_id = pu.get_device_id();
                            let device_name = pu.source.clone();
                            let stage_name = pu.get_stage_name();
                            pt.create_progress_bar(&pb_prgt_id, &device_name, &stage_name);
                        },
                        UnshackleEvent::ProgressC(pu) => {
                            // "Scanning contents" / "Copying file" / ...
                            // For DVDs and HD-DVDs there's exactly one
                            // "Copying file" task (the ISO image), for
                            // Blu-Rays there might easily be a thousand)

                            // create progress bar for current task
                            let pb_prgt_id = pu.get_device_id();
                            let pb_prgc_id = pu.get_stage_id();
                            let device_name = pu.source.clone();
                            let task_name = pu.get_task_name();
                            pt.create_progress_bar_after(&pb_prgc_id, &device_name, &task_name, &pb_prgt_id).unwrap();
                        },
                        UnshackleEvent::ProgressValue(pu) => {
                            // "Processing title sets"
                            // "Scanning contents"
                            let device_name = pu.source.clone();
                            let stage_name = pu.get_stage_name(); // reported as 'total'
                            let task_name = pu.get_task_name(); // reported as 'current'
                            let device_stage = stages.entry(device_name.clone()).or_insert(HashMap::from([(stage_name.clone(), f32::NAN)]));
                            let task_pct_old = device_stage.get(&stage_name).unwrap_or(&f32::NAN);
                            let task_pct_new = pu.prgc.percentage;

                            debug!("[{}] {}: old: {:6.2}% new: {:6.2}%", device_name, task_name, task_pct_old, task_pct_new);

                            // throttle log output: notify only if stage or
                            // percentage has changed more than 5%
                            // (prevent log-flooding)
                            if task_pct_old.is_nan() || task_pct_new >= task_pct_old + 5.0 {
                                info!("[{}] {}: {:3.0}% (done: {}, failed: {})", device_name, stage_name, task_pct_new, jobs_done, jobs_failed);
                            }

                            // throttle progress bars: notify only if stage or
                            // percentage has changed more than 0.1%
                            // (indicatif handles fine-grained throttling)
                            if task_pct_old.is_nan() || task_pct_new >= task_pct_old + 0.1 {
                                let pb_prgt_id = pu.get_device_id();
                                let pb_prgc_id = pu.get_stage_id();

                                // modify the task name to make it more
                                // obvious how stage and task are related:
                                // ----------------------------------------
                                // ⠸ [/dev/sr0] Copying all files (6%)    [█░░░░░░░░░░░░░░░░░░░] 00:03:25
                                // ⠹ [/dev/sr0] `--> Copying file (25%)   [████░░░░░░░░░░░░░░░░] 00:00:12
                                // ----------------------------------------
                                let task_name_mod = format!("`--> {}", task_name);

                                pt.update_progress_bar(&pb_prgt_id, &device_name, &stage_name, pu.prgt.percentage).unwrap();
                                pt.update_progress_bar(&pb_prgc_id, &device_name, &task_name_mod, pu.prgc.percentage).unwrap();
                            }

                            // remove the progress bar after they reached 100%
                            if pu.prgc.percentage >= 100.0 {
                                let pb_prgc_id = pu.get_stage_id();
                                pt.clear_progress_bar(&pb_prgc_id).unwrap();
                            }
                            if pu.prgt.percentage >= 100.0 {
                                let pb_prgt_id = pu.get_device_id();
                                pt.clear_progress_bar(&pb_prgt_id).unwrap();
                            }
                        },
                    }
                }
            },
            // <legacy code>
            // record completion and drain buffered events before returning
            // res = &mut th, if result.is_none() => {
            //     result = Some(res);
            // },
            // res = &mut th_mkv => {
            //     result = res;
            //     break;
            // }
            // </legacy code>
            else => {
                pt.send_text_message("tokio::select!(): break triggered");
                break;
            },
        }

        // test if the thread is still running or has finished and provided a result
        debug!("workers (pre-cleanup):  {}", workers.len());
        for (drive, worker) in workers.extract_if(|_, worker| worker.is_finished()) {
            // TODO do we need to drain potentially remaining events?
            match worker.await.unwrap() {
                Ok(result) => {
                    let elapsed = result.elapsed_secs;
                    let size_mb = (result.fs_size as f32) / 1024u32.pow(2) as f32;
                    let size_gb = (result.fs_size as f32) / 1024u32.pow(3) as f32;
                    let write_rate = size_mb / (elapsed as f32);
                    let device_id = drive.device.to_string_lossy();
                    let message = format!(
                        "[{}] Backup task completed backup after {} seconds. ({:.1}GiB written, {:.1}MiB/s)",
                        device_id, elapsed, size_gb, write_rate
                    );
                    pt.send_text_message(&message);
                    jobs_done += 1;

                    // update total bytes
                    // progress.on_backup_completed(result.fs_size);
                }
                Err(error) => {
                    let message = format!(
                        "[{}] Backup task failed: {:#?}",
                        drive.device.to_string_lossy(),
                        error.reason
                    );
                    pt.send_text_message(&message);
                    jobs_failed += 1;
                }
            }
        }
        // after this loop has run:
        // - 'workers' contains active threads
        // - 'drives_done' contains drives with successful backup
        // - 'drives_failed' contains drives with failed backup
        debug!("workers (post-cleanup): {}", workers.len());

        if workers.is_empty() {
            break;
        } else {
            sleep(Duration::from_millis(500)).await;
        }
    }

    if !rx.is_empty() {
        error!("Receiver queue contains {} messages!", rx.len());
        while let Some(msg) = rx.recv().await {
            error!("{:#?}", msg);
        }
    }

    if jobs_failed == 0 {
        exitcode::OK
    } else {
        exitcode::IOERR
    }
}

/*
pub async fn run_old(args: CmdArgs, mm: &makemkv::MakeMkv) -> i32 {
    let log_level = if let Some(level) = args.global_opts.log_level {
        level.to_string()
    } else {
        std::env::var("RUST_LOG").unwrap_or(String::from("warn"))
    };
    env_logger::Builder::new().parse_filters(&log_level).init();

    // When working with an optical drive we're 100% I/O bound. A DVD is
    // read at ~10MiB/s. A BluRay allows for slightly faster reads at
    // ~20MiB/s. This is the bottle neck. Everything else is kind of
    // irrelevant since it's very likely to be fast enough.
    //
    // The only way to reduce runtime is to use multiple optical drives and
    // run multiple extractions in parallel. Up to 6 drives are going to
    // be fine on any reasonably modern hardware.
    //
    // goal: run multiple extractions in parallel
    // design:
    // - using a dedicated process for each instance of 'makemkvcon'
    //   (mode: concurrent and parallel)
    // - using threads for the internal logic while makemkvcon is running
    //   in the background
    //   (mode: concurrent, not parallel)
    //
    // CPU usage:
    // - per 'makemkvcon backup' process: half of a CPU core (Celeron N3450)
    // - for 'ripit-cli': less than one percent

    // while running:
    // - monitor all or user-specified drives
    //   - start a worker task if a media was inserted
    // - monitor worker tasks
    // - if worker task finishes and in continuous mode
    //   - create a new worker task
    let binary = makemkvcon_bin.to_path_buf();
    let target = args.target.to_path_buf();

    info!("Using target directory: {}", target.to_string_lossy());

    for drive in &drives {
        let (tx_mkv, mut rx_mv) = mpsc::channel::<MakeMkvEvent>(256);
        let _result = makemkv.backup(drv_mkv, target, tx_mkv).await;
    }

    // let (tx, mut rx) = mpsc::channel::<UnshackleEvent>(256);

    // // first run - can't do anything until we know which drives are present
    // // - the available drives may change over time, e.g. after adding
    // //   or removing a USB drive
    // // - adding a SATA drive is possible (may require a bus scan), e.g.
    // //   `echo "- - -" > /sys/class/scsi_host/host1/scan`
    // let (tx_drv, mut rx_drv) = mpsc::channel::<DriveEvent>(256);
    // let mut drives_have = find_all_drives(makemkvcon_bin.to_path_buf(), true, tx_drv.clone()).await.unwrap();
    // let mut drives_want = find_matching_drives(&drives_have, &args.drives);

    // debug!("drives_user: {:#?}", args.drives);
    // debug!("drives_have: {:#?}", drives_have);
    // debug!("drives_want: {:#?}", drives_want);

    // // TODO figure out if it is possible for a worker to stall
    // // (worker was created but does not complete)
    // // potential scenarios:
    // // - trying to read a severely damaged disc?
    // // - ran out of disk space?
    // let max_try = 3u8;
    // let mut try_counters = HashMap::<String, u8>::new();
    // let mut workers = HashMap::new();
    // let mut workers_done = 0;
    // let mut workers_failed = 0;
    // loop {
    //     debug!("status: {}/{} active workers", workers.len(), drives_want.len());

    //     debug!("Monitoring the communication channel.");
    //     // We need to be careful with this receive channel: The transmit
    //     // channel won't close on its own causing 'rx.recv().await' to wait
    //     // forever for new messages since the channel wasn't closed. Using
    //     // 'rx.recv_many(<limit>)' won't help either since it sleeps until
    //     // at least 1 message is available. No messages = eternal slumber.
    //     //
    //     // To prevent this deadlock from happening we fetch messages until
    //     // the timeout triggers. This will give the remainder of the loop
    //     // (including 'worker.await') a chance to run.
    //     tokio::select! {
    //         Some(event) = rx.recv() => {
    //             match event {
    //                 UnshackleEvent::MsgError(x) => {
    //                     error!("[???] {}", x);
    //                 },
    //                 UnshackleEvent::MsgWarning(x) => {
    //                     warn!("[???] {}", x);
    //                 },
    //                 UnshackleEvent::MsgInfo(x) => {
    //                     info!("[???] {}", x);
    //                 },
    //                 UnshackleEvent::MsgDebug(x) => {
    //                     debug!("[???] {}", x);
    //                 },
    //                 UnshackleEvent::MsgUnknown(x) => {
    //                     info!("[???] {}", x);
    //                 },
    //                 UnshackleEvent::NeedLibreDrive(x) => {
    //                     warn!("[???] Need Libre Drive for device '{}'!", x);
    //                 },
    //                 UnshackleEvent::Status(device, file_size, write_rate) => {
    //                     let size_in_gib = file_size as f64 / f64::powf(1024.0, 3.0);
    //                     let write_rate_in_mib = write_rate as f64 / f64::powf(1024.0, 2.0);
    //                     // the formatted output is designed to stay aligned and
    //                     // provide a useful indication for the entire value range:
    //                     // --------------------------------------------------------
    //                     // Processed:  0.12 GiB (4.3 MiB/s)
    //                     // Processed:  6.22 GiB (7.3 MiB/s)
    //                     // Processed: 72.69 GiB (18.1 MiB/s)
    //                     // --------------------------------------------------------
    //                     info!(
    //                         "[{}] Processed: {:5.2} GiB ({:.1} MiB/s)",
    //                         device, size_in_gib, write_rate_in_mib
    //                     );
    //                 }
    //                 UnshackleEvent::ProgressPercentage(a, b, c, d) => {
    //                     // TODO update percentage
    //                     // TODO print percentage every x seconds
    //                 }
    //             }
    //         },
    //         _ = sleep(Duration::from_millis(5000)) => {
    //             // Timeout reached, continue to next section
    //             debug!("Event collection timeout reached.");
    //         },
    //     }

    //     debug!("[unshackle] Checking drive readiness.");
    //     if workers.is_empty() || workers.len() < drives_want.len() {
    //         // no active workers, need to manually trigger a drive update
    //         // (artificially delaying the refresh to prevent back-to-back
    //         // polling)
    //         sleep(Duration::from_secs(10)).await;
    //         debug!("No workers found. Trigger manual refresh.");
    //         drives_have = find_all_drives(makemkvcon_bin.to_path_buf(), false, tx_drv.clone()).await.unwrap();
    //         drives_want = find_matching_drives(&drives_have, &args.drives);
    //     }
    //     for drive in &drives_want {
    //         if !workers.contains_key( &drive.device_name) {
    //             let worker_try = try_counters.entry(drive.device_name.clone()).or_insert(0);
    //             if *worker_try < max_try {
    //                 // is drive ready? (disc is loaded)
    //                 match is_drive_ready(drive) {
    //                     true => {
    //                         // spawn a worker for this drive
    //                         info!("Drive '{}' is ready. Spawning worker.", drive.device_name);
    //                         let worker = spawn(unshackle_disc(
    //                             binary.clone(),
    //                             drive.clone(),
    //                             target.clone(),
    //                             // args.eject,
    //                             args.allow_overwrite,
    //                             tx.clone(),
    //                         ));
    //                         workers.insert(drive.device_name.clone(), worker);
    //                         *worker_try += 1;
    //                     },
    //                     false => {
    //                         info!("Drive '{}' is not ready.", drive.device_name);
    //                         continue;
    //                     }
    //                 }
    //             }
    //         } else {
    //             // already have a worker - ignore
    //         }
    //     }

    //     debug!("[unshackle] Checking worker progress.");
    //     let mut completed = Vec::new();
    //     for (device_name, worker) in workers.iter_mut() {
    //         if worker.is_finished() {
    //             info!("[{}] Worker has finished.", device_name);
    //             // get the result (does not incur a delay since the thread
    //             // has already finished)
    //             match worker.await {
    //                 Ok(result) => {
    //                     match result {
    //                         Ok(UnshackleResult::BackupSuccess) => {
    //                             info!("[{}] Successfully backup'd up this medium.", device_name);
    //                             workers_done += 1;
    //                             if args.continuous || args.eject {
    //                                 ripit::eject_medium(&PathBuf::from(device_name));
    //                             }
    //                         },
    //                         Ok(UnshackleResult::BackupFailure) => {
    //                             let cur_try = try_counters.get_mut(device_name).unwrap();
    //                             if *cur_try < max_try {
    //                                 error!("[{}] Failed to backup up this medium! ({}/{}) Retrying.", device_name, cur_try, max_try);
    //                                 *cur_try += 1;
    //                             } else {
    //                                 error!("[{}] Failed to backup up this medium! ({}/{}) Giving up.", device_name, cur_try, max_try);
    //                                 if args.continuous || args.eject {
    //                                     ripit::eject_medium(&PathBuf::from(device_name));
    //                                 }
    //                                 workers_failed += 1;
    //                             }
    //                         },
    //                         Err(UnshackleError::LogError(e)) => {
    //                             error!("[{}] {}.", device_name, e);
    //                         },
    //                         Err(UnshackleError::UnknownDriveStatus) => {
    //                             error!("[{}] Internal error: Unknown drive status.", device_name);
    //                         },
    //                         Err(UnshackleError::UnknownError(x)) => {
    //                             error!("[{}] Internal error: Unknown error: {}", device_name, x);
    //                         },
    //                         Err(UnshackleError::NoMedium) => {
    //                             info!("[{}] No medium in drive.", device_name);
    //                         },
    //                         // these states should not occur at this point:
    //                         Err(UnshackleError::DriveNotReady) => {
    //                             warn!("[{}] Unexpected state: Drive not ready.", device_name);
    //                         },
    //                         Err(UnshackleError::NoDrive) => {
    //                             warn!("[{}] Unexpected state: No such drive.", device_name);
    //                         },
    //                         Err(UnshackleError::BackupError(_err)) => {
    //                             // TODO decide what to do
    //                             warn!("[{}] Unexpected result: BackupError", device_name);
    //                         }
    //                     }
    //                 },
    //                 Err(x) => {
    //                     error!("[{}] Failed to await thread: {}", device_name, x);
    //                 },
    //             }

    //             // remove this worker from the work queue
    //             completed.push(device_name.clone());

    //             if args.continuous {
    //                 // reset try counter for this device to prepare for
    //                 // the next disc
    //                 try_counters.remove(device_name);
    //             }
    //         }
    //     }

    //     debug!("[unshackle] Removing completed workers.");
    //     for device_name in completed.iter() {
    //         info!("[unshackle] Removing worker for '{}'.", device_name);
    //         workers.remove(device_name);
    //     }

    //     // report
    //     let worker_count = workers.len();
    //     let drives_count = drives_want.len();
    //     if args.continuous {
    //         debug!("run(): {}/{} workers are running.", worker_count, drives_count);
    //         if worker_count < drives_count {
    //             // worker won't start if drive isn't ready
    //             // -> need to update our internal state
    //             info!("run(): Refreshing drive status.");
    //             drives_have = find_all_drives(makemkvcon_bin.to_path_buf(), false, tx_drv.clone()).await.unwrap();
    //             drives_want = find_matching_drives(&drives_have, &args.drives);
    //         }
    //     } else {
    //         let job_count = try_counters.len();
    //         if workers_done + workers_failed == job_count {
    //             if workers_failed == 0 {
    //                 info!("[unshackle] All workers have finished successfully.");
    //                 return exitcode::OK;
    //             } else {
    //                 warn!("[unshackle] Some workers have failed.");
    //                 return exitcode::IOERR;
    //             }
    //         } else {
    //             debug!("run(): {} workers are running, {} total jobs.", worker_count, job_count);
    //         };
    //     }
    // }

    exitcode::OK
}
*/

// ------------------------------------------------------------------------
// private helper functions
// ------------------------------------------------------------------------

// <none>

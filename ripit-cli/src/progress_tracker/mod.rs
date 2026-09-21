/*
    Progress rendering for unshackle command

    Handles both interactive (TTY) and non-interactive output modes with graceful
    degradation. In TTY mode, uses indicatif's multi-progress for in-place table
    updates. In pipe/file mode, uses structured log format.
*/

mod stage;

// standard library imports
use std::collections::HashMap;

// third-party imports
use indicatif::{MultiProgress, ProgressBar, ProgressStyle};
#[allow(unused_imports)]
use log::{debug, error, info, trace, warn};

// crate-provided imports
// <none>

// ------------------------------------------------------------------------
// public interface
// ------------------------------------------------------------------------

pub use stage::Stage;

/// manages multi-device progress bars (rendered using indicatif)
/// <https://github.com/console-rs/indicatif/blob/main/examples/multi.rs>
pub struct ProgressTracker {
    mp: MultiProgress,
    style: ProgressStyle,

    pb: HashMap<String, ProgressBar>,
}

impl ProgressTracker {
    pub fn new() -> Self {
        Self {
            mp: MultiProgress::new(),
            pb: HashMap::new(),
            style: create_progress_style(),
        }
    }

    pub fn get_mp(&self) -> MultiProgress {
        self.mp.clone()
    }

    pub fn create_progress_bar(&mut self, id: &str, device: &str, stage: &str) {
        let message = format!("[{}] {:40}", device, stage);
        let length = 100; // 100% = done

        // !! order of operations is extremely important !!
        // see indicatif's documentation for details:
        // <https://github.com/console-rs/indicatif/blob/92cf1ae5f47c4bc2fb83556aa96f021727082ccc/src/multi.rs#L25-L29>
        let pb = self
            .mp
            .add(ProgressBar::new(length))
            .with_style(self.style.clone())
            .with_message(message);

        self.pb.insert(id.to_owned(), pb);
    }

    pub fn create_progress_bar_after(
        &mut self,
        id: &str,
        device: &str,
        stage: &str,
        id_parent: &str,
    ) -> Result<(), bool> {
        let message = format!("[{}] {:40}", device, stage);
        let length = 100; // 100% = done

        if let Some(pb_parent) = self.pb.get_mut(id_parent) {
            let pb = self
                .mp
                .insert_after(pb_parent, ProgressBar::new(length))
                .with_style(self.style.clone())
                .with_message(message);

            self.pb.insert(id.to_owned(), pb);
            Ok(())
        } else {
            Err(false)
        }
    }

    pub fn update_progress_bar(
        &mut self,
        id: &str,
        device: &str,
        stage: &str,
        percentage: f32,
    ) -> Result<(), bool> {
        let stage_str = if percentage.is_nan() {
            stage.to_string()
        } else {
            format!("{} ({:.0}%)", stage, percentage)
        };
        let message = format!("{:40} {:40}", device, stage_str);

        if let Some(pb) = self.pb.get_mut(id) {
            pb.set_message(message);
            pb.set_position(percentage.floor() as u64);
            Ok(())
        } else {
            Err(false)
        }
    }

    pub fn clear_progress_bar(&mut self, id: &str) -> Result<(), bool> {
        if let Some(pb) = self.pb.get_mut(id) {
            pb.finish_and_clear();
            Ok(())
        } else {
            Err(false)
        }
    }

    pub fn send_text_message(&mut self, message: &str) {
        self.mp.println(message).unwrap();
    }
}

// ------------------------------------------------------------------------
// private helper functions
// ------------------------------------------------------------------------

/// create the progress style for indicatif bars
fn create_progress_style() -> ProgressStyle {
    ProgressStyle::default_bar()
        .template("{spinner:.green} {msg:80} [{bar:20.cyan/blue}] {elapsed_precise}")
        .unwrap()
        .tick_strings(&["⠋", "⠙", "⠹", "⠸", "⠼", "⠴", "⠦", "⠧", "⠇", "⠏"])
}

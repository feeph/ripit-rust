/*!
    track stage progress (PRGT & PRGC)
*/

// standard library imports
use std::time::Instant;

// third-party imports
#[allow(unused_imports)]
use log::{debug, error, info, trace, warn};

// crate-provided imports
// <none>

// ------------------------------------------------------------------------
// public interface
// ------------------------------------------------------------------------

pub struct Stage {
    prgt_pct: f32,
    prgc_pct: f32,
    time_beg: Instant,
    time_end: Option<Instant>,
}

impl Stage {
    pub fn new() -> Stage {
        Stage {
            prgt_pct: f32::NAN,
            prgc_pct: f32::NAN,
            time_beg: Instant::now(),
            time_end: None,
        }
    }

    pub fn get_prgc(&self) -> f32 {
        self.prgc_pct
    }

    pub fn get_prgt(&self) -> f32 {
        self.prgt_pct
    }

    pub fn get_elapsed(&self) -> u64 {
        let time_end = match self.time_end {
            Some(time_end) => time_end,
            None => Instant::now(),
        };

        time_end.duration_since(self.time_beg).as_secs()
    }

    pub fn update_progress(&mut self, prgt: f32, prgc: f32) {
        self.prgt_pct = prgt;
        self.prgc_pct = prgc;

        if prgt == 100.00 {
            self.time_end = Some(Instant::now());
        }
    }
}

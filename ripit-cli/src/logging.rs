//! Custom logging module for ripit-cli
//!
//! ## Overview
//!
//! This module provides a custom logger implementation that routes log messages to
//! stdout or stderr based on severity level, with custom formatting and color support.
//!
//! ## Design Principles
//!
//! ### Message Routing
//! - **INFO, DEBUG, TRACE**: routed to stdout
//! - **WARN, ERROR**: routed to stderr
//!
//! This allows callers to easily separate normal application output (stdout) from
//! warnings and errors (stderr), enabling better shell redirection and piping.
//!
//! ### Message Formatting
//! Each log message is formatted as: `<SEVERITY>: <message>`
//!
//! Where `<SEVERITY>` is a single character:
//! - `I` for Info
//! - `W` for Warning
//! - `E` for Error
//! - `D` for Debug
//! - `T` for Trace
//!
//! ### Color Support
//! Colors are used to enhance readability when the terminal supports it:
//! - **Yellow** (`\x1b[33m`) for WARN level messages
//! - **Red** (`\x1b[31m`) for ERROR level messages
//! - **Default terminal color** for all other levels
//!
//! Color codes are automatically disabled when the output is not a TTY (e.g., piped
//! to a file), thanks to the `env_logger::WriteStyle::Auto` configuration.
//!
//! ### Module-Based Filtering
//!
//! The logger applies different filtering rules based on whether `--log-level` was
//! explicitly provided by the user:
//!
//! **Default Behavior** (no `--log-level` flag):
//! - `cmd_unshackle` and `ripit_cli` modules: show Info level and above
//! - `ripit` crate module: show Info level and above
//! - All other modules: show only Warn level and above
//! - This minimizes noise from third-party crates while showing relevant application logs
//!
//! **With `--log-level` Flag**:
//! - The specified level applies to **all modules** uniformly
//! - Example: `--log-level debug` shows Debug and above from all modules
//! - Example: `--log-level info` shows Info and above from all modules
//! - Useful for debugging issues in dependencies or the entire application
//!
//! ### Thread Safety
//! The logger is thread-safe and suitable for use in async contexts (tokio, etc.).
//! Each log call is independent and does not hold locks across await points.

use log::{Level, LevelFilter, Record};
use std::io::Write;

/// Custom logger implementation that implements the `log::Log` trait.
struct CustomLogger {
    filter: LevelFilter,
}

impl log::Log for CustomLogger {
    fn enabled(&self, metadata: &log::Metadata) -> bool {
        metadata.level() <= self.filter
    }

    fn log(&self, record: &Record) {
        if !self.enabled(record.metadata()) {
            return;
        }

        // Determine if we should show this message based on module and level
        let show_msg = should_show_message(record);
        if !show_msg {
            return;
        }

        let level_char = match record.level() {
            Level::Trace => "T",
            Level::Debug => "D",
            Level::Info => "I",
            Level::Warn => "W",
            Level::Error => "E",
        };

        let (color_code, reset_code) = match record.level() {
            Level::Warn => ("\x1b[33m", "\x1b[0m"),  // Yellow
            Level::Error => ("\x1b[31m", "\x1b[0m"), // Red
            _ => ("", ""),                           // Default terminal color
        };

        let message = format!(
            "{}{}{}: {}",
            color_code,
            level_char,
            reset_code,
            record.args()
        );

        // Route to stdout for Info/Debug/Trace, stderr for Warn/Error
        match record.level() {
            Level::Info | Level::Debug | Level::Trace => {
                let _ = writeln!(std::io::stdout(), "{}", message);
            }
            Level::Warn | Level::Error => {
                let _ = writeln!(std::io::stderr(), "{}", message);
            }
        }
    }

    fn flush(&self) {}
}

/// Determine if a message should be displayed based on module and level.
///
/// This implements the module-based filtering strategy described in the module documentation.
fn should_show_message(record: &Record) -> bool {
    let module = record.module_path().unwrap_or("");

    // ripit's command line interface: show info, warnings and errors
    if module.starts_with("ripit_cli") {
        return record.level() <= Level::Warn;
    }

    // other modules: show warnings and errors
    record.level() <= Level::Warn
}

/// Initialize the logger with the specified log level.
///
/// # Arguments
///
/// * `log_level` - The desired log level. If `None`, uses the default filtering strategy.
///   If `Some(level)`, applies that level uniformly to all modules.
///
/// # Examples
///
/// ```ignore
/// // Use default filtering (selective, minimal noise)
/// init_logger(None);
///
/// // Show Debug level and above from all modules
/// init_logger(Some(log::Level::Debug));
///
/// // Show only errors
/// init_logger(Some(log::Level::Error));
/// ```
pub fn init_logger(log_level: Option<Level>) {
    let filter = log_level
        .map(|l| l.to_level_filter())
        .unwrap_or(LevelFilter::Info);

    let logger = CustomLogger { filter };

    log::set_boxed_logger(Box::new(logger))
        .map(|()| log::set_max_level(filter))
        .expect("Failed to set logger");
}

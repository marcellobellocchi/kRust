//! Kernel logging
//! right now, logs go all into serial port. later a console sink can be added in _log.

use core::fmt;
use core::sync::atomic::{AtomicU8, Ordering};

/// Message severity, from most to least severe
#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
#[repr(u8)]
pub enum Level {
    Error,
    Warn,
    Info,
    Debug,
    Trace,
}

impl Level {
    fn label(self) -> &'static str {
        match self {
            Level::Error => "ERROR",
            Level::Warn => "WARN",
            Level::Info => "INFO",
            Level::Debug => "DEBUG",
            Level::Trace => "TRACE",
        }
    }
}

static MAX_LEVEL: AtomicU8 = AtomicU8::new(if cfg!(debug_assertions) {
    Level::Debug as u8
} else {
    Level::Info as u8
});

pub fn set_level(level: Level) {
    MAX_LEVEL.store(level as u8, Ordering::Relaxed);
}

#[doc(hidden)]
pub fn _log(level: Level, module: &str, args: fmt::Arguments) {
    if level as u8 > MAX_LEVEL.load(Ordering::Relaxed) {
        return;
    }
    crate::drivers::serial::_print(format_args!("[{} {}] {}\n", level.label(), module, args));
}

#[macro_export]
macro_rules! error {
    ($($arg:tt)*) => { $crate::log::_log($crate::log::Level::Error, module_path!(), format_args!($($arg)*)) };
}

#[macro_export]
macro_rules! warn {
    ($($arg:tt)*) => { $crate::log::_log($crate::log::Level::Warn, module_path!(), format_args!($($arg)*)) };
}

#[macro_export]
macro_rules! info {
    ($($arg:tt)*) => { $crate::log::_log($crate::log::Level::Info, module_path!(), format_args!($($arg)*)) };
}

#[macro_export]
macro_rules! debug {
    ($($arg:tt)*) => { $crate::log::_log($crate::log::Level::Debug, module_path!(), format_args!($($arg)*)) };
}

#[macro_export]
macro_rules! trace {
    ($($arg:tt)*) => { $crate::log::_log($crate::log::Level::Trace, module_path!(), format_args!($($arg)*)) };
}

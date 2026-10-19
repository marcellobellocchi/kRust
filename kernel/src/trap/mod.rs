//! Architecture-independent error handling
//! Architecture layer catches exceptions in its own hardware-specific way, decodes them
//! into an exception and calls handle. What is decided after an exception is thrown
//! is declared here and is shared by every arch

use core::fmt;
use crate::arch::TrapFrame;

/// Privilege level the CPU was running at during exception
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Mode {
    Kernel,
    User,
}

/// Kind of memaccess that caused page fault
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Access {
    Read,
    Write,
    Execute,
}

#[derive(Debug, Clone, Copy)]
pub struct PageFault {
    /// Virtual address that was accessed
    pub address: usize,
    pub access: Access,
    pub mode: Mode,

    /// true: page is mapped but access wasn't allowed (e.g. write to read-only)
    /// false: nothing is mapped at that address
    pub page_present: bool,
}

/// An exception, as described to generic code
#[derive(Debug, Clone, Copy)]
pub enum Exception {
    /// Software breakpoint (debugger trap). Execution can continue.
    Breakpoint,
    /// Single-step or hardware watchpoint. Execution can continue.
    Debug,
    DivideError,
    IllegalInstruction,
    FloatingPoint,
    AlignmentFault,
    PageFault(PageFault),
    /// Unrecoverable, wit architecture-specific cause described for the log
    Fatal {
        description: &'static str,
        error_code: Option<u64>,
    },
}

impl fmt::Display for PageFault {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        let access = match self.access {
            Access::Read => "read from",
            Access::Write => "write to",
            Access::Execute => "instruction fetch from",
        };
        let state = if self.page_present { "protected page" } else { "unmapped page" };
        let mode = match self.mode {
            Mode::Kernel => "kernel",
            Mode::User => "user",
        };
        write!(f, "{access} {state} at {:#x} ({mode} mode)", self.address)
    }
}

impl fmt::Display for Exception {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self {
            Exception::Breakpoint => write!(f, "breakpoint"),
            Exception::Debug => write!(f, "debug trap"),
            Exception::DivideError => write!(f, "divide error"),
            Exception::IllegalInstruction => write!(f, "illegal instruction"),
            Exception::FloatingPoint => write!(f, "floating-point error"),
            Exception::AlignmentFault => write!(f, "misaligned access"),
            Exception::PageFault(fault) => write!(f, "page fault: {fault}"),
            Exception::Fatal { description, error_code: Some(code) } => {
                write!(f, "{description} (error code {code:#x})")
            }
            Exception::Fatal { description, error_code: None } => write!(f, "{description}"),
        }
    }
}

/// Entry point for every exception, called on by arch layer
/// Returns only if execution can resume at, or after, the interrupted instruction
/// else it panics
pub fn handle(exception: Exception, frame: &TrapFrame) {
    match exception {
        Exception::Breakpoint | Exception::Debug => {
            info!("{exception}\n{frame}")
        }
        // Later: a fault in user mode kills (or signals) the process instead of
        // letting the whole os collapse, and some exceptions can be resolved
        _ => panic!("unhandled exception: {exception}\n{frame}"),
    }
}
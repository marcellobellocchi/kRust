//! x86_64 exception handlers: each one decodes the hardware state into a
//! generic Exception and hands it to trap::handle. No policy lives here.
//!
//! Handlers use the `x86-interrupt` calling convention (nightly feature
//! `abi_x86_interrupt`): the compiler generates the register saving and the
//! `iretq`, so no assembly stubs are needed.

use core::arch::asm;
use core::fmt;

use crate::trap::{self, Access, Exception, Mode, PageFault};

// what the CPU pushes onto the stack were an exception to occur
#[derive(Debug, Clone, Copy)]
#[repr(C)]
pub struct TrapFrame {
    pub rip: u64,
    pub cs: u64,
    pub rflags: u64,
    pub rsp: u64,
    pub ss: u64,
}

impl TrapFrame {
    pub fn instruction_pointer(&self) -> usize {
        self.rsp as usize
    }

    pub fn stack_pointer(&self) -> usize {
        self.rsp as usize
    }

    /// The low 2 bits of CS are the privilege level the CPU was running at
    pub fn mode(&self) -> Mode {
        if self.cs & 3 == 0 { Mode::Kernel } else { Mode::User }
    }
}

impl fmt::Display for TrapFrame {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(
            f,
            "  rip={:#018x} rsp={:#018x}\n cs={:#06x} ss={:#06x} rflags={:#010x}",
            self.rip, self.rsp, self.cs, self.ss, self.rflags
        )
    }
}

// Handlers

fn fatal(description: &'static str, error_code: Option<u64>, frame: &TrapFrame) {
    trap::handle(Exception::Fatal { description, error_code }, frame);
}

pub extern "x86-interrupt" fn divide_error(frame: TrapFrame) {
    trap::handle(Exception::DivideError, &frame);
}

pub extern "x86-interrupt" fn debug(frame: TrapFrame) {
    trap::handle(Exception::Debug, &frame);
}

pub extern "x86-interrupt" fn non_maskable(frame: TrapFrame) {
    fatal("non-maskable interrupt", None, &frame);
}

pub extern "x86-interrupt" fn breakpoint(frame: TrapFrame) {
    trap::handle(Exception::Breakpoint, &frame);
}

pub extern "x86-interrupt" fn overflow(frame: TrapFrame) {
    fatal("overflow (INTO)", None, &frame);
}

pub extern "x86-interrupt" fn bound_range(frame: TrapFrame) {
    fatal("BOUND range exceeded", None, &frame);
}

pub extern "x86-interrupt" fn invalid_opcode(frame: TrapFrame) {
    trap::handle(Exception::IllegalInstruction, &frame);
}

pub extern "x86-interrupt" fn device_not_available(frame: TrapFrame) {
    fatal("FPU/SIMD unit not available", None, &frame);
}

/// Runs on the IST stack set up in `gdt.rs`, so it works even after a stack overflow.
pub extern "x86-interrupt" fn double_fault(frame: TrapFrame, error_code: u64) -> ! {
    fatal("double fault", Some(error_code), &frame);
    panic!("double fault handler returned");
}

pub extern "x86-interrupt" fn invalid_tss(frame: TrapFrame, error_code: u64) {
    fatal("invalid TSS", Some(error_code), &frame);
}

pub extern "x86-interrupt" fn segment_not_present(frame: TrapFrame, error_code: u64) {
    fatal("segment not present", Some(error_code), &frame);
}

pub extern "x86-interrupt" fn stack_segment(frame: TrapFrame, error_code: u64) {
    fatal("stack segment fault", Some(error_code), &frame);
}

pub extern "x86-interrupt" fn general_protection(frame: TrapFrame, error_code: u64) {
    // Error code is a segment selector when a segment caused it, 0 otherwise
    fatal("general protection fault", Some(error_code), &frame);
}

pub extern "x86-interrupt" fn page_fault(frame: TrapFrame, error_code: u64) {
    const PRESENT: u64 = 1 << 0;
    const WRITE: u64 = 1 << 1;
    const USER: u64 = 1 << 2;
    const INSTRUCTION_FETCH: u64 = 1 << 4;

    let fault = PageFault {
        address: read_cr2(),
        access: if error_code & INSTRUCTION_FETCH != 0 {
            Access::Execute
        } else if error_code & WRITE != 0 {
            Access::Write
        } else {
            Access::Read
        },
        mode: if error_code & USER != 0 { Mode::User } else { Mode::Kernel },
        page_present: error_code & PRESENT != 0,
    };
    trap::handle(Exception::PageFault(fault), &frame);
}

pub extern "x86-interrupt" fn x87_floating_point(frame: TrapFrame) {
    trap::handle(Exception::FloatingPoint, &frame);
}

pub extern "x86-interrupt" fn alignment_check(frame: TrapFrame, _error_code: u64) {
    trap::handle(Exception::AlignmentFault, &frame);
}

pub extern "x86-interrupt" fn machine_check(frame: TrapFrame) -> ! {
    fatal("machine check (hardware error)", None, &frame);
    panic!("machine check handler returned");
}

pub extern "x86-interrupt" fn simd_floating_point(frame: TrapFrame) {
    trap::handle(Exception::FloatingPoint, &frame);
}

/// The CPU stores the faulting address of a page fault in CR2.
fn read_cr2() -> usize {
    let value: usize;
    unsafe { asm!("mov {}, cr2", out(reg) value, options(nomem, nostack, preserves_flags)) };
    value
}

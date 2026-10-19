#[cfg(target_arch = "x86_64")]
mod x86_64;
#[cfg(target_arch = "x86_64")]
use x86_64 as imp;

pub use imp::{init, TrapFrame, inb, outb};
mod boot;
mod gdt;
mod idt;
mod exceptions;
mod port;

pub use exceptions::TrapFrame;
pub use port::{inb, outb};

pub fn init() {
    unsafe {
        gdt::init();
        idt::init();
    }
}
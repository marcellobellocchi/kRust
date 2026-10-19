#![no_std]
#![no_main]
use core::arch::asm;
#[macro_use]
mod drivers;
#[macro_use]
mod log;
mod arch;
mod panic;

#[unsafe(no_mangle)]
pub extern "C" fn kmain(_multiboot_info: usize, _magic_num: u32) -> ! {
    drivers::serial::init();
    drivers::serial::write_str("Hello, world!");

    loop {
        unsafe {asm!("hlt")}
    }
}
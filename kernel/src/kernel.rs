#![no_std]
#![no_main]
#![feature(abi_x86_interrupt)]
use core::arch::asm;
#[macro_use]
mod drivers;
#[macro_use]
mod log;
mod trap;
mod arch;
mod panic;

#[unsafe(no_mangle)]
pub extern "C" fn kmain(_multiboot_info: usize, _magic_num: u32) -> ! {
    drivers::serial::init();
    arch::init();

    loop {
        unsafe {asm!("hlt")}
    }
}
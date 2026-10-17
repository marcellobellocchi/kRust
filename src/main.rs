#![no_std]
#![no_main]

mod serial;
mod boot;

use core::arch::asm;
use core::panic::PanicInfo;

#[unsafe(no_mangle)]
pub extern "C" fn kmain(_multiboot_info: usize) -> ! {
    serial::init();
    serial::write_str("Hello, world!");

    loop {
        unsafe {asm!("hlt")}
    }
}

#[panic_handler]
fn panic(_info: &PanicInfo) -> ! {
    serial::write_str("KERNEL PANIC");
    loop {
        unsafe {asm!("hlt")}
    }
}
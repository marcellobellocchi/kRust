use crate::arch::{inb, outb};
use core::fmt::{self, Write};
use core::sync::atomic::{AtomicBool, Ordering};

//first serial port fixed location
const COM1: u16 = 0x3F8;
static PRESENT: AtomicBool = AtomicBool::new(false);

pub fn init() -> bool {
    unsafe {
        outb(COM1 + 1, 0x00);   //disable interrupts
        outb(COM1 + 3, 0x80);   //set DLAB
        outb(COM1 + 0, 0x03);   //speed divisor low byte
        outb(COM1 + 1, 0x00);   //speed divisor high byte
        outb(COM1 + 3, 0x03);   //clear dlab, 8N1
        outb(COM1 + 2, 0xC7);   //enable and clear FIFO

        //Sanity check: anything sent comes back+
        outb(COM1 + 4, 0x1E);
        outb(COM1 + 0, 0xAE);
        let ok = inb(COM1 + 0) == 0xAE;

        outb(COM1 + 4, 0x0B);   //QEMU-needed modem control lines (DTR, RTS, OUT2)
        PRESENT.store(ok, Ordering::Relaxed);
        ok
    }
}

///Send one character
pub fn write_byte(byte: u8) {
    if !PRESENT.load(Ordering::Relaxed) {
        return;
    }
    unsafe {
        while inb(COM1 + 5) & 0x20 == 0 {}  //wait until port ready
        outb(COM1, byte);
    }
}

///Send ASCII string
pub fn write_str(s: &str) {
    for byte in s.bytes() {
        if byte == b'\n' {
            write_byte(b'\r');
        }
        write_byte(byte);
    }
}

pub struct SerialWriter;

impl Write for SerialWriter {
    fn write_str(&mut self, s: &str) -> fmt::Result {
        write_str(s);
        Ok(())
    }
}

#[doc(hidden)]
pub fn _print(args: fmt::Arguments) {
    SerialWriter.write_fmt(args).ok();
}

#[macro_export]
macro_rules! serial {
    ($($arg:tt)*) => { $crate::drivers::serial::_print(format_args!($($arg)*)) };
}

#[macro_export]
macro_rules! serialln {
    () => { serial!("\n") };
    ($($arg:tt)*) => { serial!("{}\n", format_args!($($arg)*)) };
}
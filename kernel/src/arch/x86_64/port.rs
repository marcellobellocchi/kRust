use core::arch::asm;

/// Write one byte to an I/O port
pub unsafe fn outb(port: u16, val: u8) {
    unsafe { asm!("out dx, al", in("dx") port, in("al") val, options(nostack, preserves_flags)); }
}

/// Read one byte from an I/O port
pub unsafe fn inb(port: u16) -> u8 {
    let val: u8;
    unsafe { asm!("in al, dx", in("dx") port, out("al") val, options(nostack, preserves_flags), ); }
    val
}
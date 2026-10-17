use core::arch::asm;

//first serial port fixed location
const COM1: u16 = 0x3F8;

///Send one byte to a port
unsafe fn outb(port: u16, val: u8) {
    unsafe {
        asm!(
        //send value in al to port in dx
        "out dx, al",
        in("dx") port,
        in("al") val,

        //compiler optimization options
        options(nomem, nostack, preserves_flags)
        );
    }
}

unsafe fn inb(port: u16) -> u8 {
    let val: u8;
    unsafe {
        asm!(
            //read from port dx into al
            "in al, dx",
            in("dx") port,
            out("al") val,

            //compiler optimization options
            options(nomem, nostack, preserves_flags),
        );
    }
    val
}

pub fn init() {
    unsafe {
        outb(COM1 + 1, 0x00);   //disable interrupts
        outb(COM1 + 3, 0x80);   //set DLAB
        outb(COM1 + 0, 0x03);   //speed divisor low byte
        outb(COM1 + 1, 0x00);   //speed divisor high byte
        outb(COM1 + 3, 0x03);   //clear dlab, 8N1
        outb(COM1 + 2, 0xC7);   //enable and clear FIFO
        outb(COM1 + 4, 0x0B);   //QEMU-needed modem control lines (DTR, RTS, OUT2)
    }
}

///Send one character
pub fn write_byte(byte: u8) {
    unsafe {
        while inb(COM1 + 5) & 0x20 == 0 {}  //wait until port ready
        outb(COM1, byte);
    }
}

///Send ASCII string
pub fn write_str(s: &str) {
    for byte in s.bytes() {
        write_byte(byte);
    }
}
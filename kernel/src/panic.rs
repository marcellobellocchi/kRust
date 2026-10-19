use core::arch::asm;
use core::panic::PanicInfo;

#[panic_handler]
fn panic(info: &PanicInfo) -> ! {
    serialln!("KERNEL PANIC: {}", info);
    loop {
        unsafe { asm!("cli; hlt", options(nomem, nostack)) }
    }
}
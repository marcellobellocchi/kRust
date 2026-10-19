mod multiboot2;
mod tables;
use core::arch::global_asm;

global_asm!(include_str!("boot.s")); //Import assembly for 32 to 64 bit switch

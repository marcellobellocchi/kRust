ENTRY(_start)

SECTIONS {
    . = 2M;
    .multiboot : { KEEP(*(.multiboot_header)) }
    .text      : { *(.text .text.*) }
    .rodata    : { *(.rodata .rodata.*) }
    .data      : { *(.data .data.*) }
    .bss       : { *(.bss .bss.*) }
}
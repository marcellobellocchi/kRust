ENTRY(_start)

SECTIONS {
    . = 2M;
    __kernel_start = .;

    /* Must be within the first 32 KiB of the file, so it goes first */
    .multiboot : { KEEP(*(.multiboot_header)) }

    /* Each region starts on a 4 KiB page, so it can later get its own permissions */
    .text ALIGN(4K) : { *(.text .text.*) }
    .rodata ALIGN(4K) : { *(.rodata .rodata.*) }
    .data ALIGN(4K) : {
        *(.data.rel.ro .data.rel.ro.*)
        *(.data .data.*)
    }
    .bss ALIGN(4K) : { *(.bss .bss.*) }

    . = ALIGN(4K);
    __kernel_end = .;

    /* Unwind tables and compiler notes are useless with panic=abort */
    /DISCARD/ : { *(.eh_frame*) *(.comment) *(.note*) }
}
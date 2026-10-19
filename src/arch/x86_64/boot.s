.section .text
.code32
.global _start
_start:
    mov edi,  ebx                   #multiboot2 info ptr
    mov esi, eax                    #mutliboot2 magic

    mov eax,  offset P4
    mov cr3,  eax                   # Link to top-level table for cpu

    # Enable PAE, long mode and paging
    mov eax,  cr4
    or  eax,  1 << 5                # PAE
    mov cr4,  eax
    mov ecx,  0xC0000080            # IA32_EFER (long mode)
    rdmsr
    or  eax, 1 << 8                 # LME
    wrmsr
    mov eax, cr0
    or  eax, 1 << 31                # Paging
    mov cr0, eax

    # Load GDT, far-jump into 64-bit code segment
    lgdt [GDT_PTR]
    ljmp 0x8, offset long_mode_start

.code64
long_mode_start:
    xor ax,  ax
    mov ss,  ax
    mov ds,  ax
    mov es,  ax
    mov rsp, offset STACK + 16384
    mov edi, edi                  # zero-extend info ptr
    cld
    call kmain                    # rdi is now the one holding info pointer

2:
    hlt
    jmp 2b

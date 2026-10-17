.section .text
.code32
.global _start
_start:
    mov esp,  offset STACK + 16384
    mov edi,  ebx                   #save to edi the multiboot2 info pointer grub generated in ebx

    #P2 filled at compile time, P3, P4 need link time addresses filled here
    mov eax,  offset P3 + 3         # 3 = present | writable
    mov [P4], eax                   # P4[0] -> P3
    mov eax,  offset P2 + 3
    mov [P3], eax                   # P3[0] -> P2
    mov eax,  offset P4
    mov cr3,  eax                   # Link to top-level table for cpu

    # Enable PAE (cr4 bit 5), long mode (EFER bit 8) and paging (cr0 bit 31)
    mov eax,  cr4
    or  eax,  1 << 5
    mov cr4,  eax
    mov ecx,  0xC0000080            # MSR number of IA32_EFER
    rdmsr
    or  eax, 1 << 8
    wrmsr
    mov eax, cr0
    or  eax, 1 << 31
    mov cr0, eax

    # Load GDT, far-jump into 64-bit code segment
    mov eax, offset GDT
    mov [GDT_PTR + 2], eax
    lgdt [GDT_PTR]
    ljmp 0x8, offset long_mode_start

.code64
long_mode_start:
    xor ax,  ax
    mov ss,  ax
    mov ds,  ax
    mov es,  ax
    mov rsp, offset STACK + 16384 # Null selector for data segments
    call kmain                    # rdi is now the one holding info pointer

2:                                # kmain should never return. If it does, halt forever.
    hlt
    jmp 2b
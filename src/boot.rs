use core::arch::global_asm;

//Import assembly for 32 to 64 bit switch
global_asm!(include_str!("arch/x86_64/boot.s"));

#[repr(C, align(8))]
struct Multiboot2Header([u32; 6]);

//Compiler cant see assembly using some vars,
//#[used] prevents them from being removed
#[used]
#[unsafe(link_section = ".multiboot_header")]
static MULTIBOOT2_HEADER: Multiboot2Header = Multiboot2Header([
    0xe85250d6,                                             //Multiboot2 magic number
    0,                                                      //architecture: i386
    24,                                                     //header lenght (bytes)
    0u32.wrapping_sub(0xe85250d6u32.wrapping_add(24)),  //checksum
    0,                                                      //end tags
    8,
]);

#[repr(C, align(4096))]
struct PageTable([u64; 512]);

//Build P2 table at compile time: 1 GiB identity mapped with huge pages
const fn identity_p2() -> PageTable {
    let mut entries = [0u64; 512];
    let mut i = 0;
    while i < 512 {
        entries[i] = ((i as u64) << 21) | 0b1000_0011;
        i += 1;
    }
    PageTable(entries)
}

//P3, P4 filled by boot.s
#[used]
#[unsafe(no_mangle)]
static mut P4: PageTable = PageTable([0; 512]);

#[used]
#[unsafe(no_mangle)]
static mut P3: PageTable = PageTable([0; 512]);

#[used]
#[unsafe(no_mangle)]
static mut P2: PageTable = identity_p2();

//first entry mandatory null descriptor
//second is 64-bit code segment
#[used]
#[unsafe(no_mangle)]
static GDT: [u64; 2] = [0, 0x00AF9A000000FFFF];

//limit is size -1 because base is written at runtime
#[repr(C, packed)]
struct GdtPointer {
    limit: u16,
    base: u64,
}

#[used]
#[unsafe(no_mangle)]
static mut GDT_PTR: GdtPointer = GdtPointer { limit: 15, base: 0 };

//16KiB, 16-byte alignment as neede by SysV ABI
#[repr(C, align(16))]
struct Stack([u8; 16384]);

#[used]
#[unsafe(no_mangle)]
static mut STACK: Stack = Stack([0; 16384]);
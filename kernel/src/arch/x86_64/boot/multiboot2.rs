//Multiboot conf

#[repr(C, align(8))]
struct Multiboot2Header([u32; 6]);

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

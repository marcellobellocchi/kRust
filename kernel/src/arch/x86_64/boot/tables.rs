// Paging conf

#[repr(C, align(4096))]
struct PageTable([PageTableEntry; 512]);

/// An 8-byte page table entry. Both fields occupy the same bytes:
/// 'value' for entries known at compile time,
/// 'next_table' for those holding another table address, which the linker fills in.
#[derive(Clone, Copy)]
#[repr(C)]
union PageTableEntry {
    value: u64,
    next_table: *const u8,
}

const PRESENT: u64 = 1 << 0;
const WRITABLE: u64 = 1 << 1;
const HUGE_PAGE: u64 = 1 << 7; //at least I'm funny

/// Number of GiB mapped at boot
const IDENTITY_MAPPED_GIB: usize = 4;
const EMPTY_ENTRY: PageTableEntry = PageTableEntry { value: 0 };

/// Entry pointing to lower-level table (ex. P4 -> P3). Flags added via
/// ptr arithmetic, so the linker emits 'address + flags'
const fn next_table_entry(table: *const PageTable) -> PageTableEntry {
    PageTableEntry { next_table: table.cast::<u8>().wrapping_add((PRESENT | WRITABLE) as usize) }
}

/// P2 identity-mapping the 'gib'-th GiB with 2 MiB huge pages.
const fn identity_p2(gib: usize) -> PageTable {
    let mut entries = [EMPTY_ENTRY; 512];
    let mut i = 0;
    while i < 512 {
        let address = ((gib as u64) << 30) | ((i as u64) << 21);
        entries[i] = PageTableEntry { value: address | HUGE_PAGE | WRITABLE | PRESENT };
        i += 1;
    }
    PageTable(entries)
}

const fn identity_p2s() -> [PageTable; IDENTITY_MAPPED_GIB] {
    let mut tables = [const { PageTable([EMPTY_ENTRY; 512]) }; IDENTITY_MAPPED_GIB];
    let mut gib = 0;
    while gib < IDENTITY_MAPPED_GIB {
        tables[gib] = identity_p2(gib);
        gib += 1;
    }
    tables
}

#[unsafe(no_mangle)]
static mut P4: PageTable = {
    let mut entries = [EMPTY_ENTRY; 512];
    entries[0] = next_table_entry(&raw const P3);
    PageTable(entries)
};

static mut P3: PageTable = {
    let mut entries = [EMPTY_ENTRY; 512];
    let mut gib = 0;
    while gib < IDENTITY_MAPPED_GIB {
        entries[gib] = next_table_entry((&raw const P2S).cast::<PageTable>().wrapping_add(gib));
        gib += 1;
    }
    PageTable(entries)
};
static mut P2S: [PageTable; IDENTITY_MAPPED_GIB] = identity_p2s();



// GDT conf

//first entry mandatory null descriptor
//second is 64-bit code segment
#[unsafe(no_mangle)]
static GDT: [u64; 2] = [0, 0x00AF9B000000FFFF];

#[repr(C, packed)]
struct GdtPointer {
    limit: u16,
    base: *const [u64; 2],
}
unsafe impl Sync for GdtPointer {}

#[unsafe(no_mangle)]
static mut GDT_PTR: GdtPointer = GdtPointer {
    limit: size_of::<[u64; 2]>() as u16 -1,
    base: &raw const GDT,
};

//16KiB, 16-byte alignment as needed by SysV ABI
#[repr(C, align(16))]
struct Stack([u8; 16384]);

#[unsafe(no_mangle)]
static mut STACK: Stack = Stack([0; 16384]);
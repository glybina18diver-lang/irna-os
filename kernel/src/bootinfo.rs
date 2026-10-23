// kernel/src/bootinfo.rs — разбор структуры Multiboot2, переданной GRUB (M1).
// Спецификация: Multiboot2 (public standard) — тег-структура, выровненная по 8.
use core::ffi::c_void;

extern "C" {
    static mb_mboot_ptr: u32; // сохранён trampoline (arch.asm): EBX от GRUB
    static mb_mboot_ebx: u32; // дубль EBX — диагностика
}

const MB2_BOOT_MAGIC: u32 = 0x36D7_6289;

#[repr(C)]
struct TagHeader {
    tag: u32,
    size: u32,
}

#[repr(C)]
struct MemoryArea {
    base_addr: u64,
    length: u64,
    kind: u32,
}

#[repr(C)]
struct MemmapTag {
    header: TagHeader,
    entry_size: u32,
    entry_version: u32,
    // далее memory_area[entry_size / 24]
}

/// Итог разбора memory map загрузчика.
/// Диагностика: сырые значения указателя MBI, сохранённые trampoline'ом.
pub fn raw_ptrs() -> (u64, u64) {
    unsafe { (mb_mboot_ptr as u64, mb_mboot_ebx as u64) }
}

pub struct BootInfo {
    /// Конец первой непрерывной свободной области, начинающейся с 1 МиБ.
    /// Все кадры [1MiB, free_mem_top) доступны физическому аллокатору.
    pub free_mem_top: u64,
    /// Всего свободной памяти (тег type=1), байт.
    pub total_free: u64,
    /// Диагностика парсера: найден ли MBI и какие теги встречены.
    pub mbi_found: bool,
    /// Битовая маска тегов < 64, встретившихся при разборе (для serial-дампа).
    pub tags_seen: u64,
}

/// Разбор MBI. Вызывается ОДИН раз на раннем этапе, до включения прерываний.
pub fn parse() -> BootInfo {
    let mut info = BootInfo { free_mem_top: 0, total_free: 0, mbi_found: false, tags_seen: 0 };
    let mbi = unsafe { &raw const mb_mboot_ptr } as *const u32;
    if mbi.is_null() {
        return info;
    }
    unsafe {
        let magic = *mbi;
        if magic != MB2_BOOT_MAGIC {
            return info; // не наш формат — аллокатор использует fallback-предел
        }
        info.mbi_found = true;
        let total = *mbi.add(1) as usize;
        let mut off = 8usize;
        while off + 8 <= total {
            let tag = (mbi as *const u8).add(off) as *const TagHeader;
            let t = (*tag).tag;
            let sz = (*tag).size as usize;
            if sz < 8 || off + sz > total {
                break;
            }
            if t == 6 {
                // basicmemmap
                let mm = tag as *const MemmapTag;
                let esize = (*mm).entry_size as usize;
                if esize >= 24 {
                    let count = (sz - 16) / esize;
                    let base = (mm as *const u8).add(16);
                    for i in 0..count {
                        let a = base.add(i * esize) as *const MemoryArea;
                        let kind = (*a).kind;
                        let begin = (*a).base_addr;
                        let len = (*a).length;
                        if kind == 1 && len > 0 {
                            info.total_free += len;
                            // ищем область, смежную с 1 МиБ (типично: usable 1MiB..)
                            if begin <= 0x10_0000 && begin + len > info.free_mem_top
                                && begin + len <= 0x4000_0000
                            {
                                info.free_mem_top = begin + len;
                            }
                        }
                    }
                }
            }
            if t < 64 {
                info.tags_seen |= 1u64 << t;
            }
            off += (sz + 7) & !7;
        }
    }
    info
}

/// Подтверждение c_void для будущего использования (загрузочные модули — M2).
#[allow(dead_code)]
pub type ModulePtr = *const c_void;

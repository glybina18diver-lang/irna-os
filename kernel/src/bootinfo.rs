// kernel/src/bootinfo.rs — разбор структуры Multiboot2, переданной GRUB (M1).
// Спецификация: Multiboot2 (public standard) — тег-структура, выровненная по 8.
use core::ffi::c_void;

extern "C" {
    static mb_mboot_ptr: u32; // arch.asm: адрес КОПИИ MBI (безопасный буфер)
    static mb_mboot_ebx: u32; // arch.asm: исходный EBX от GRUB (диагностика)
    static mb_mboot_magic: u32; // arch.asm: EAX от GRUB (0x36D76289)
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

/// Тег 8 (framebuffer). Поля после `reserved` присутствуют только для type=1.
#[repr(C)]
struct FramebufferTag {
    header: TagHeader,
    addr: u64,
    pitch: u32,
    width: u32,
    height: u32,
    bpp: u8,
    fb_type: u8,
    reserved: u16,
    red_pos: u8,
    red_size: u8,
    green_pos: u8,
    green_size: u8,
    blue_pos: u8,
    blue_size: u8,
}

/// Линейный framebuffer, отданный загрузчиком (type=1, RGB).
#[derive(Clone, Copy)]
pub struct FrameBuffer {
    pub addr: u64,
    pub pitch: u32,
    pub width: u32,
    pub height: u32,
    pub bpp: u8,
    pub red_pos: u8,
    pub red_size: u8,
    pub green_pos: u8,
    pub green_size: u8,
    pub blue_pos: u8,
    pub blue_size: u8,
}

/// Диагностика: адрес копии MBI, исходный EBX и magic из EAX.
pub fn raw_ptrs() -> (u64, u64, u64) {
    unsafe {
        (
            mb_mboot_ptr as u64,
            mb_mboot_ebx as u64,
            mb_mboot_magic as u64,
        )
    }
}

pub struct BootInfo {
    /// Конец первой непрерывной свободной области, начинающейся с 1 МиБ.
    /// Все кадры [1MiB, free_mem_top) доступны физическому аллокатору.
    pub free_mem_top: u64,
    /// Всего свободной памяти (тег type=1), байт.
    pub total_free: u64,
    /// Диагностика парсера: найден ли MBI и какие теги встречены.
    pub mbi_found: bool,
    /// total_size из заголовка MBI (offset 0), байт.
    pub mbi_size: u32,
    /// Битовая маска тегов < 64, встретившихся при разборе (для serial-дампа).
    pub tags_seen: u64,
    /// Линейный framebuffer (тег 8, type=1). `None` — графика недоступна.
    pub fb: Option<FrameBuffer>,
}

/// Разбор MBI. Вызывается ОДИН раз на раннем этапе, до включения прерываний.
pub fn parse() -> BootInfo {
    let mut info = BootInfo {
        free_mem_top: 0,
        total_free: 0,
        mbi_found: false,
        mbi_size: 0,
        tags_seen: 0,
        fb: None,
    };
    // ВАЖНО: читаем ЗНАЧЕНИЕ статической переменной (адрес копии MBI),
    // а не адрес самой переменной. Раньше здесь был `&raw const mb_mboot_ptr`,
    // из-за чего парсер читал память самой переменной, а не структуру MBI.
    let mbi_addr = unsafe { mb_mboot_ptr } as usize;
    let magic = unsafe { mb_mboot_magic };
    if mbi_addr == 0 || magic != MB2_BOOT_MAGIC {
        return info; // не наш формат — аллокатор использует fallback-предел
    }
    let mbi = mbi_addr as *const u32;
    unsafe {
        // Multiboot2: offset 0 = total_size, offset 4 = reserved,
        // offset 8 = первый тег. Magic в структуре отсутствует (он в EAX).
        let total = *mbi as usize;
        if total < 16 || total > 8192 {
            return info; // повреждённый/неправдоподобный размер
        }
        info.mbi_found = true;
        info.mbi_size = total as u32;
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
            if t == 8 {
                // framebuffer: берём только RGB (type=1) с 24/32 bpp
                let ft = tag as *const FramebufferTag;
                if sz >= 38 && (*ft).fb_type == 1 {
                    let bpp = (*ft).bpp;
                    if bpp == 24 || bpp == 32 {
                        info.fb = Some(FrameBuffer {
                            addr: (*ft).addr,
                            pitch: (*ft).pitch,
                            width: (*ft).width,
                            height: (*ft).height,
                            bpp,
                            red_pos: (*ft).red_pos,
                            red_size: (*ft).red_size,
                            green_pos: (*ft).green_pos,
                            green_size: (*ft).green_size,
                            blue_pos: (*ft).blue_pos,
                            blue_size: (*ft).blue_size,
                        });
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

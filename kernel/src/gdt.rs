// kernel/src/gdt.rs — GDT + TSS (M1).
// Trampoline (arch.asm) загружает минимальную GDT для far-jmp в long mode.
// Здесь строится полная таблица: code64(0x08), data(0x10), user-code(0x18|3),
// user-data(0x20|3), TSS(0x28, 16-байтный sys-дескриптор). Загрузка — через
// asm-обвязку gdt_reload в arch.asm (far jmp корректнее писать в NASM).

use core::ptr::addr_of;

#[repr(C, packed)]
#[derive(Clone, Copy)]
pub struct Tss {
    reserved1: u32,
    pub rsp: [u64; 3], // привилегированные стеки RSP0..RSP2
    reserved2: u64,
    pub ist: [u64; 7], // interrupt stacks; IST1 = ist[0] (#DF)
    reserved3: u64,
    reserved4: u16,
    iomap_base: u16,
}

impl Tss {
    const ZERO: Tss = Tss {
        reserved1: 0,
        rsp: [0; 3],
        reserved2: 0,
        ist: [0; 7],
        reserved3: 0,
        reserved4: 0,
        iomap_base: (core::mem::size_of::<Tss>() as u16) + 1,
    };
}

/// Таблица: 5 обычных дескрипторов по 8 байт + 16-байтный TSS-дескриптор
/// (tss_lo/tss_hi идут ПОСЛЕДНИМИ, чтобы не перекрывать поле `tss`).
#[repr(C, align(16))]
struct GdtData {
    null: u64,
    code64: u64,
    data: u64,
    user_code: u64,
    user_data: u64,
    tss_lo: u64,
    tss_hi: u64,
    pub tss: Tss,
}

const fn code64_desc() -> u64 {
    (1 << 0) | (1 << 1) | (0xF << 16) | (1 << 41) | (1 << 43) | (1 << 44) | (1 << 47) | (1 << 53)
}
const fn data_desc() -> u64 {
    (1 << 0) | (1 << 1) | (0xF << 16) | (1 << 41) | (1 << 44) | (1 << 47)
}

static mut GDT: GdtData = GdtData {
    null: 0,
    code64: code64_desc(),
    data: data_desc(),
    user_code: code64_desc() | (3 << 45), // DPL=3
    user_data: data_desc() | (3 << 45),   // DPL=3
    tss_lo: 0,
    tss_hi: 0,
    tss: Tss::ZERO,
};

pub const SEL_CODE64: u16 = 0x08;
pub const SEL_DATA: u16 = 0x10;
pub const SEL_USER_CODE: u16 = 0x18 | 3;
pub const SEL_USER_DATA: u16 = 0x20 | 3;
pub const SEL_TSS: u16 = 0x28;

extern "C" {
    fn gdt_reload(ptr_val: u64, sel_tss: u16); // arch.asm: lgdt + ltr + far jmp + сегменты
}

/// Инициализация полной GDT+TSS.
/// `kernel_stack_top` — вершина стека ядра (RSP0), `df_stack_top` — IST1 (#DF).
pub unsafe fn init(kernel_stack_top: u64, df_stack_top: u64) {
    let g = core::ptr::addr_of_mut!(GDT);
    let tss_addr = addr_of!((*g).tss) as u64;
    let tss_limit = (core::mem::size_of::<Tss>() - 1) as u64;

    (*g).tss.rsp[0] = kernel_stack_top;
    (*g).tss.ist[0] = df_stack_top;

    // AMD64 sys-дескриптор TSS (16 байт): lo-qword byte0-1 limit[15:0];
    // byte2-4 base[23:0]; byte5 access(0x89 avail 64-bit TSS);
    // byte6 flags(0)|limit[19:16](TSS limit < 64KiB => 0); byte7 base[31:24].
    // hi-qword: base[63:32].
    let lo: u64 = (tss_limit & 0xFFFF)
        | ((tss_addr & 0xFF_FFFF) << 16)
        | (0x89u64 << 40)
        | (((tss_addr >> 24) & 0xFF) << 56);
    (*g).tss_lo = lo;
    (*g).tss_hi = tss_addr >> 32;

    // псевдо-дескриптор lgdt (10 байт: limit u16 + base u64 — формат 64-bit)
    #[repr(C, packed)]
    struct P {
        limit: u16,
        base: u64,
    }
    let ptr = P {
        limit: (core::mem::size_of::<GdtData>() - core::mem::size_of::<Tss>() - 1) as u16,
        base: g as *const GdtData as u64,
    };
    gdt_reload(&ptr as *const P as u64, SEL_TSS);
}

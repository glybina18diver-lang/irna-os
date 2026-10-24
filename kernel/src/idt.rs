// kernel/src/idt.rs — IDT и обработчики исключений (M1).
// 32 gate-дескриптора (векторы 0..31) указывают на isr_stub_* в arch.asm.
// Общий stub сохраняет регистры и вызывает exception_handler с указателем на
// InterruptFrame. #BP (int3) используется как smoke-тест: обработчик печатает
// вектор и возвращается через iretq; остальные исключения останавливают ядро.

use core::arch::asm;
use core::mem::size_of;

use crate::serial;

#[repr(C, packed)]
#[derive(Clone, Copy)]
struct IdtEntry {
    offset_low: u16,
    selector: u16,
    ist: u8,
    type_attr: u8,
    offset_mid: u16,
    offset_high: u32,
    zero: u32,
}

impl IdtEntry {
    const ZERO: IdtEntry = IdtEntry {
        offset_low: 0,
        selector: 0,
        ist: 0,
        type_attr: 0,
        offset_mid: 0,
        offset_high: 0,
        zero: 0,
    };
}

static mut IDT: [IdtEntry; 256] = [IdtEntry::ZERO; 256];

extern "C" {
    static isr_stub_table: [u64; 32];
}

const SEL_CODE: u16 = 0x08;
const GATE_INTERRUPT: u8 = 0x8E; // present | DPL=0 | 64-bit interrupt gate

/// Кадр, который формирует isr_common (порядок совпадает с push в arch.asm).
#[repr(C)]
pub struct InterruptFrame {
    pub r15: u64,
    pub r14: u64,
    pub r13: u64,
    pub r12: u64,
    pub r11: u64,
    pub r10: u64,
    pub r9: u64,
    pub r8: u64,
    pub rbp: u64,
    pub rdi: u64,
    pub rsi: u64,
    pub rdx: u64,
    pub rcx: u64,
    pub rbx: u64,
    pub rax: u64,
    pub vector: u64,
    pub error_code: u64,
    pub rip: u64,
    pub cs: u64,
    pub rflags: u64,
    pub rsp: u64,
    pub ss: u64,
}

fn set_gate(vector: usize, handler: u64, ist: u8, attr: u8) {
    let idt = unsafe { &mut *core::ptr::addr_of_mut!(IDT) };
    let e = &mut idt[vector];
    e.offset_low = handler as u16;
    e.selector = SEL_CODE;
    e.ist = ist & 0x7;
    e.type_attr = attr;
    e.offset_mid = (handler >> 16) as u16;
    e.offset_high = (handler >> 32) as u32;
    e.zero = 0;
}

/// Загрузить IDT. Вызывать после gdt::init() (селектор 0x08 должен быть валиден).
pub fn init() {
    let table = unsafe { &isr_stub_table };
    for i in 0..32 {
        // #DF (8) идёт на IST1, остальные — на текущий стек.
        let ist = if i == 8 { 1 } else { 0 };
        set_gate(i, table[i], ist, GATE_INTERRUPT);
    }

    #[repr(C, packed)]
    struct IdtPointer {
        limit: u16,
        base: u64,
    }
    let ptr = IdtPointer {
        limit: (size_of::<[IdtEntry; 256]>() - 1) as u16,
        base: core::ptr::addr_of!(IDT) as u64,
    };
    unsafe {
        asm!(
            "lidt [{}]",
            in(reg) core::ptr::addr_of!(ptr),
            options(readonly, nostack, preserves_flags)
        );
    }
}

fn vector_name(v: u64) -> &'static str {
    match v {
        0 => "#DE divide error",
        1 => "#DB debug",
        2 => "NMI",
        3 => "#BP breakpoint",
        4 => "#OF overflow",
        5 => "#BR bound range",
        6 => "#UD invalid opcode",
        7 => "#NM device not available",
        8 => "#DF double fault",
        10 => "#TS invalid TSS",
        11 => "#NP segment not present",
        12 => "#SS stack fault",
        13 => "#GP general protection",
        14 => "#PF page fault",
        16 => "#MF x87 floating point",
        17 => "#AC alignment check",
        18 => "#MC machine check",
        19 => "#XM SIMD floating point",
        20 => "#VE virtualization",
        21 => "#CP control protection",
        29 => "#VC VMM communication",
        30 => "#SX security",
        _ => "reserved/unknown",
    }
}

#[unsafe(no_mangle)]
pub extern "C" fn exception_handler(frame: *const InterruptFrame) {
    let f = unsafe { &*frame };

    serial::write_str("\nEXCEPTION ");
    serial::write_hex_u64(f.vector);
    serial::write_str(" ");
    serial::write_str(vector_name(f.vector));
    serial::write_str(" error=");
    serial::write_hex_u64(f.error_code);
    serial::write_str(" rip=");
    serial::write_hex_u64(f.rip);
    serial::write_str(" cs=");
    serial::write_hex_u64(f.cs);
    serial::write_str(" rflags=");
    serial::write_hex_u64(f.rflags);

    if f.vector == 14 {
        let cr2: u64;
        unsafe { asm!("mov {}, cr2", out(reg) cr2) };
        serial::write_str(" cr2=");
        serial::write_hex_u64(cr2);
    }
    serial::write_str("\n");

    if f.vector == 3 {
        // int3 — trap: RIP уже указывает на следующую инструкцию.
        serial::write_str("IDT smoke: #BP handled, returning\n");
        return;
    }

    serial::write_str("KERNEL HALT (unhandled exception)\n");
    loop {
        unsafe { asm!("cli", "hlt") };
    }
}

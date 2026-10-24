// kernel/src/main.rs — точка входа ядра «ИрНа».
// M0: hello-world в serial. M1: разбор Multiboot2 memory map, физический
// аллокатор кадров, GDT+TSS, куча ядра, текстовый вывод в VGA-консоль.
#![no_std]
#![no_main]

extern crate alloc;

mod bootinfo;
mod brand;
mod gdt;
mod heap;
mod idt;
mod pmm;
mod serial;
mod vga;

use brand::KERNEL_NAME;

#[unsafe(no_mangle)]
pub extern "C" fn kernel_main() -> ! {
    // --- ранний канал: serial (работает без памяти) ---
    serial::init();
    serial::write_str("==================================================\n");
    // Имя ОС — только из константы brand.rs (реестр branding.md, строка B3). D-008.
    serial::write_str(KERNEL_NAME);
    serial::write_str(" kernel: M1 boot start\n");

    // --- 1. память загрузчика: разбираем MBI (EBX сохранён trampoline'ом) ---
    let info = bootinfo::parse();
    let (p1, p2, magic) = bootinfo::raw_ptrs();
    serial_line("MBI magic saved  = ", magic);
    serial_line("MBI ptr copy     = ", p1);
    serial_line("MBI ptr ebx orig = ", p2);
    serial_line("MBI total size   = ", info.mbi_size as u64);
    serial_line("MBI found        = ", info.mbi_found as u64);
    serial_line("MBI tags mask    = ", info.tags_seen);
    serial_line("MBI free_mem_top = ", info.free_mem_top);
    serial_line("MBI total_free   = ", info.total_free);
    if info.free_mem_top == 0 {
        serial::write_str("WARN: MBI memory map не найдена — PMM использует fallback\n");
    }

    // --- 2. физический аллокатор кадров ---
    unsafe { pmm::init(&info) };
    let (free, total) = unsafe { pmm::stats() };
    serial_line("PMM frames total = ", total as u64);
    serial_line("PMM frames free  = ", free as u64);
    //Smoke-тест: выделить/освободить кадр
    let f0 = unsafe { pmm::alloc_frame() };
    match f0 {
        Some(addr) => {
            serial::write_str("PMM alloc_frame OK\n");
            unsafe { pmm::free_frame(addr) };
        }
        None => serial::write_str("PMM alloc_frame FAILED\n"),
    }

    // --- 3. GDT + TSS (полная таблица вместо trampoline-минимума) ---
    extern "C" {
        static stack_top_addr: u8;
        static df_stack_top_addr: u8;
    }
    unsafe {
        gdt::init(&stack_top_addr as *const u8 as u64, &df_stack_top_addr as *const u8 as u64);
    }
    serial::write_str("GDT+TSS loaded (RSP0, IST1)\n");

    // --- 3b. IDT + обработчики исключений ---
    idt::init();
    serial::write_str("IDT loaded (32 exception gates)\n");
    // Smoke-тест: int3 -> #BP -> exception_handler -> iretq.
    unsafe { core::arch::asm!("int3") };
    serial::write_str("IDT int3 smoke OK\n");

    // --- 4. куча ядра (bump-пул 1 МиБ) + проверка через global_allocator ---
    {
        use core::alloc::{GlobalAlloc, Layout};
        let p = unsafe { heap::ALLOCATOR.alloc(Layout::from_size_align(100, 8).unwrap()) };
        if !p.is_null() {
            unsafe { core::ptr::write_bytes(p, 0x5A, 100) };
            serial::write_str("kernel heap alloc OK (100 bytes)\n");
        } else {
            serial::write_str("kernel heap alloc FAILED\n");
        }
    }

    // --- 5. VGA-текстовая консоль (0xB8000, identity-map) ---
    unsafe { vga::clear() };
    vga::println("ИрНа OS — этап M1: базовое ядро");
    vga::println("-----------------------------");
    vga::print("brand: ");
    vga::println(KERNEL_NAME);
    {
        use alloc::string::ToString;
        let s = alloc::format!("mem usable top: {} KiB", info.free_mem_top / 1024);
        vga::println(s.as_str());
        let s2 = alloc::format!("frames: {} free / {} total", free, total);
        vga::println(s2.as_str());
        let _ = "".to_string();
    }
    vga::println("heap: 1 MiB bump pool");
    vga::println("VGA text console OK");
    serial::write_str("VGA text console OK\n");

    serial::write_str(KERNEL_NAME);
    serial::write_str(" kernel: M1 subsystems OK (bootinfo/PMM/GDT/heap/VGA)\n");
    serial::write_str("==================================================\n");
    loop {
        unsafe { core::arch::asm!("hlt") };
    }
}

// --- утилиты форматирования чисел для serial (без std) ---
static mut ITOA_BUF: [u8; 24] = [0; 24];

fn serial_line(label: &str, val: u64) {
    serial::write_str(label);
    // utoa в статический буфер с конца
    let buf = unsafe { &mut *core::ptr::addr_of_mut!(ITOA_BUF) };
    let mut n = val;
    let mut i = buf.len();
    loop {
        i -= 1;
        buf[i] = b'0' + (n % 10) as u8;
        n /= 10;
        if n == 0 {
            break;
        }
    }
    serial::write_str(core::str::from_utf8(&buf[i..]).unwrap_or("?"));
    serial::write_str("\n");
}

#[panic_handler]
fn panic(info: &core::panic::PanicInfo) -> ! {
    serial::write_str("\nKERNEL PANIC\n");
    {
        use core::fmt::Write as _;
        struct SerialWriter;
        impl fmt::Write for SerialWriter {
            fn write_str(&mut self, s: &str) -> core::fmt::Result {
                serial::write_str(s);
                Ok(())
            }
        }
        let _ = SerialWriter.write_fmt(format_args!("{}", info));
        serial::write_str("\n");
    }
    loop {
        unsafe { core::arch::asm!("cli", "hlt") };
    }
}

use core::fmt;

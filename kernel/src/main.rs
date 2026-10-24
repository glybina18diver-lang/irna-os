// kernel/src/main.rs — точка входа ядра «ИрНа».
// M0: hello-world в serial. M1: разбор Multiboot2 memory map, физический
// аллокатор кадров, GDT+TSS, куча ядра, текстовый вывод в VGA-консоль.
#![no_std]
#![no_main]

extern crate alloc;

mod bootinfo;
mod brand;
mod console;
mod fb;
mod font;
mod gdt;
mod heap;
mod idt;
mod io;
mod irq;
mod keyboard;
mod paging;
mod pic;
mod pit;
mod pmm;
mod serial;
mod syscall;
mod user;
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
    serial::write_str("IDT loaded (48 gates: 32 exception + 16 IRQ)\n");
    // Smoke-тест: int3 -> #BP -> interrupt_handler -> iretq.
    unsafe { core::arch::asm!("int3") };
    serial::write_str("IDT int3 smoke OK\n");

    // --- 3c. PIC + PIT: аппаратные прерывания ---
    pic::init();
    serial::write_str("PIC remapped (IRQ 0x20..0x2F)\n");
    pit::init();
    serial::write_str("PIT programmed (100 Hz)\n");
    pic::unmask(0); // IRQ0 — таймер
    unsafe { core::arch::asm!("sti") };
    serial::write_str("interrupts enabled (IF=1)\n");
    // 20 тиков при 100 Гц — примерно 200 мс; заодно проверяем, что IRQ0
    // действительно приходит и EOI отправляется (иначе второй тик не придёт).
    pit::wait_ticks(20);
    serial::write_str("PIT: 20 ticks OK\n");
    if irq::unexpected() != 0 {
        serial::write_str("WARN: unexpected/spurious IRQs seen\n");
    }

    // --- 3d. PS/2 клавиатура (IRQ1) ---
    keyboard::init();
    pic::unmask(1);
    serial::write_str("PS/2 keyboard ready (IRQ1)\n");

    // --- 3e. видеоконсоль: LFB из тега 8, иначе VGA text ---
    let fb_ok = match info.fb {
        Some(f) => {
            serial_line("LFB addr         = ", f.addr);
            serial_line("LFB pitch        = ", f.pitch as u64);
            serial_line("LFB width        = ", f.width as u64);
            serial_line("LFB height       = ", f.height as u64);
            serial_line("LFB bpp          = ", f.bpp as u64);
            let ok = fb::init(&f);
            if ok {
                let (w, h, cols, rows) = fb::stats();
                serial::write_str("framebuffer console OK (");
                serial::write_dec_u64(w as u64);
                serial::write_str("x");
                serial::write_dec_u64(h as u64);
                serial::write_str(", ");
                serial::write_dec_u64(cols as u64);
                serial::write_str("x");
                serial::write_dec_u64(rows as u64);
                serial::write_str(" chars)\n");
            } else {
                serial::write_str("WARN: LFB не отображён — остаёмся в VGA text\n");
            }
            ok
        }
        None => {
            serial::write_str("LFB нет в MBI — VGA text fallback\n");
            false
        }
    };

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

    // --- 5. текстовая консоль: LFB или VGA text ---
    console::clear();
    console::println("ИрНа OS — этап M1: базовое ядро");
    console::println("-----------------------------");
    console::print("brand: ");
    console::println(KERNEL_NAME);
    {
        use alloc::string::ToString;
        let s = alloc::format!("mem usable top: {} KiB", info.free_mem_top / 1024);
        console::println(s.as_str());
        let s2 = alloc::format!("frames: {} free / {} total", free, total);
        console::println(s2.as_str());
        let _ = "".to_string();
    }
    console::println("heap: 1 MiB bump pool");
    console::println("IDT: 48 gates, PIC: 0x20..0x2F, PIT: 100 Hz");
    console::println("Кириллица: проверка шрифта PT Mono (10x16)");
    if fb_ok {
        console::println("framebuffer console OK");
        serial::write_str("framebuffer console OK\n");
    } else {
        console::println("VGA text console OK");
        serial::write_str("VGA text console OK\n");
    }

    // --- 6. SYSCALL/SYSRET: MSR + smoke-тест из ring 3 ---
    unsafe { syscall::init() };
    serial::write_str("SYSCALL/SYSRET enabled (EFER.SCE, STAR/LSTAR/FMASK)\n");
    console::println("syscall: write/exit, тест ring 3");

    serial::write_str(KERNEL_NAME);
    serial::write_str(" kernel: M1 subsystems OK (bootinfo/PMM/GDT/IDT/PIC/PIT/heap/LFB/syscall)\n");
    serial::write_str("==================================================\n");

    // ring 3 -> syscall write -> syscall exit -> kernel_after_user -> echo_loop
    user::run_test()
}

/// Цикл эха клавиатуры. Вызывается из kernel_after_user после возврата
/// из ring 3 (и был бы вызван из kernel_main, если бы тест возвращался).
pub fn echo_loop() -> ! {
    serial::write_str("keyboard echo: нажмите клавиши (serial + ");
    serial::write_str(if fb::is_active() { "framebuffer" } else { "VGA" });
    serial::write_str(")\n");

    // IRQ1 кладёт скан-коды в буфер, здесь мы их разбираем и эхо-выводим.
    // hlt будит процессор по любому прерыванию.
    loop {
        while let Some(k) = keyboard::poll() {
            match k {
                keyboard::Key::Char(c) => {
                    serial::write_str("key: '");
                    serial::write_char(c);
                    serial::write_str("'\n");
                    console::putc(c);
                }
                keyboard::Key::Space => {
                    serial::write_str("key: Space\n");
                    console::putc(b' ');
                }
                keyboard::Key::Enter => {
                    serial::write_str("key: Enter\n");
                    console::putc(b'\n');
                }
                keyboard::Key::Backspace => {
                    serial::write_str("key: Backspace\n");
                    console::putc(0x08);
                }
                other => {
                    serial::write_str("key: ");
                    serial::write_str(other.name());
                    serial::write_str("\n");
                }
            }
        }
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

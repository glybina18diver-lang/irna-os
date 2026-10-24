// kernel/src/io.rs — порты ввода-вывода x86-64 (M1).
// Тонкая обёртка над in/out, чтобы pic.rs и pit.rs не дублировали asm!.

use core::arch::asm;

/// Записать байт в порт.
#[inline]
pub unsafe fn outb(port: u16, value: u8) {
    unsafe {
        asm!(
            "out dx, al",
            in("dx") port,
            in("al") value,
            options(nomem, nostack, preserves_flags)
        );
    }
}

/// Прочитать байт из порта.
#[inline]
pub unsafe fn inb(port: u16) -> u8 {
    let value: u8;
    unsafe {
        asm!(
            "in al, dx",
            out("al") value,
            in("dx") port,
            options(nomem, nostack, preserves_flags)
        );
    }
    value
}

/// Небольшая задержка для старых контроллеров (запись в неиспользуемый порт 0x80).
#[inline]
pub unsafe fn io_wait() {
    unsafe { outb(0x80, 0) };
}

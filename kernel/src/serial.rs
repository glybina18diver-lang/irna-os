// kernel/src/serial.rs — драйвер COM1 (16550 UART) для вывода логов в QEMU.
// Порт 0x3F8; инициализация по стандартной процедуре (DLAB, скорость 115200, 8N1).
use core::arch::asm;

const COM1: u16 = 0x3F8;

/// Порт-вывод (используется также vga.rs для CRT-регистров).
#[inline]
pub unsafe fn outb(port: u16, value: u8) {
    asm!("out dx, al", in("dx") port, in("al") value, options(nomem, nostack, preserves_flags));
}

#[inline]
unsafe fn inb(port: u16) -> u8 {
    let value: u8;
    // al — фиксированный регистр порта ввода x86 (обязателен для `in`)
    asm!("in al, dx", in("dx") port, out("al") value, options(nomem, nostack, preserves_flags));
    value
}

pub fn init() {
    unsafe {
        outb(COM1 + 1, 0x00); // отключить прерывания UART
        outb(COM1 + 3, 0x80); // DLAB=1 — разрешить установку скорости
        outb(COM1 + 0, 0x01); // делитель скорости: младший байт (115200 / 1 = 115200 бод)
        outb(COM1 + 1, 0x00); // старший байт
        outb(COM1 + 3, 0x03); // DLAB=0, 8 бит данных, без чётности, 1 стоп-бит (8N1)
        outb(COM1 + 2, 0xC7); // включить/очистить FIFO
        outb(COM1 + 4, 0x0B); // DTR + RTS +_OUT2 (необходимо для прерываний, у нас — polling)
    }
}

fn write_byte(byte: u8) {
    // ждём освобождения передатчика (LSR bit 5 = TX empty)
    unsafe {
        while inb(COM1 + 5) & 0x20 == 0 {
            core::hint::spin_loop();
        }
        outb(COM1, byte);
    }
}

/// Вывод произвольных байтов (UTF-8 допустим — терминал QEMU покажет как сможет;
/// для serial-логов гарантируем читаемость имени ОС через hex-дамп при необходимости).
pub fn write_bytes(bytes: &[u8]) {
    for &b in bytes {
        if b == b'\n' {
            write_byte(b'\r');
        }
        write_byte(b);
    }
}

pub fn write_str(s: &str) {
    write_bytes(s.as_bytes());
}

/// Вывести один байт как символ (для эха клавиатуры).
pub fn write_char(c: u8) {
    write_byte(c);
}

/// Вывод u64 в hex с префиксом 0x (без ведущих нулей).
pub fn write_hex_u64(v: u64) {
    const HEX: &[u8; 16] = b"0123456789abcdef";
    write_str("0x");
    let mut started = false;
    for i in (0..16).rev() {
        let nib = ((v >> (i * 4)) & 0xF) as usize;
        if nib != 0 || started || i == 0 {
            started = true;
            write_byte(HEX[nib]);
        }
    }
}

/// Хексадамп строки — надёжный способ увидеть UTF-8 (кириллицу) в сырых логах.
pub fn write_str_hex(label: &str, s: &str) {
    write_str(label);
    write_str(": ");
    static HEX: &[u8; 16] = b"0123456789abcdef";
    for &b in s.as_bytes() {
        write_byte(HEX[(b >> 4) as usize]);
        write_byte(HEX[(b & 0xF) as usize]);
    }
    write_str("\n");
}

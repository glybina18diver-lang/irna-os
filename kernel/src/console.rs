// kernel/src/console.rs — единая точка текстового вывода (M1).
//
// Если загрузчик дал линейный framebuffer — печатаем в него (fb.rs),
// иначе используется VGA text mode (vga.rs). Serial-вывод остаётся отдельным:
// он работает всегда и служит журналом для CI/тестов.

use crate::fb;
use crate::vga;

pub fn clear() {
    if fb::is_active() {
        fb::clear();
    } else {
        vga::clear();
    }
}

pub fn print(s: &str) {
    if fb::is_active() {
        fb::print(s);
    } else {
        vga::print(s);
    }
}

pub fn println(s: &str) {
    print(s);
    print("\n");
}

/// Один байт (эхо клавиатуры: ASCII, \n, \r, backspace).
pub fn putc(c: u8) {
    if fb::is_active() {
        fb::put_char(c as char);
    } else {
        vga::putc(c);
    }
}

/// Один символ UTF-8 (например, кириллица из будущей раскладки).
#[allow(dead_code)]
pub fn put_char(c: char) {
    if fb::is_active() {
        fb::put_char(c);
    } else {
        vga::print(c.encode_utf8(&mut [0u8; 4]));
    }
}

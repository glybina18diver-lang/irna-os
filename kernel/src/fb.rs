// kernel/src/fb.rs — текстовая консоль в линейном framebuffer (M1).
//
// Пиксели пишутся напрямую в LFB (identity-map, RW — см. paging.rs).
// Шрифт — 10x16 (PT Mono, см. font.rs); на экранах >= 700 px по высоте
// включается масштаб 2x, иначе 1x. Прокрутка — сдвиг строк пикселей вверх.
//
// Поддерживается 32- и 24-битный RGB (позиции/размеры полей берутся из тега
// Multiboot2, поэтому порядок каналов не предполагается).

use crate::bootinfo::FrameBuffer;
use crate::font;

const BG: u32 = 0x0010_1820; // тёмно-синий фон
const FG: u32 = 0x00E8_E8E8; // светло-серый текст

struct Fb {
    addr: u64,
    pitch: usize,
    width: usize,
    height: usize,
    bytes_pp: usize,
    scale: usize,
    cols: usize,
    rows: usize,
    col: usize,
    row: usize,
    red_pos: u8,
    red_size: u8,
    green_pos: u8,
    green_size: u8,
    blue_pos: u8,
    blue_size: u8,
}

static mut FB: Option<Fb> = None;

pub fn is_active() -> bool {
    unsafe { FB.is_some() }
}

/// Инициализация консоли. Возвращает false, если LFB недоступен/не отображён.
pub fn init(info: &FrameBuffer) -> bool {
    if info.bpp != 32 && info.bpp != 24 {
        return false;
    }
    if info.width == 0 || info.height == 0 || info.pitch == 0 {
        return false;
    }
    let size = info.pitch as u64 * info.height as u64;
    if !crate::paging::map_identity_2m(info.addr, size) {
        return false;
    }

    let scale = if info.height >= 700 { 2 } else { 1 };
    let cols = (info.width as usize) / (font::GLYPH_W * scale);
    let rows = (info.height as usize) / (font::GLYPH_H * scale);
    if cols == 0 || rows == 0 {
        return false;
    }

    unsafe {
        FB = Some(Fb {
            addr: info.addr,
            pitch: info.pitch as usize,
            width: info.width as usize,
            height: info.height as usize,
            bytes_pp: (info.bpp / 8) as usize,
            scale,
            cols,
            rows,
            col: 0,
            row: 0,
            red_pos: info.red_pos,
            red_size: info.red_size,
            green_pos: info.green_pos,
            green_size: info.green_size,
            blue_pos: info.blue_pos,
            blue_size: info.blue_size,
        });
    }
    clear();
    true
}

fn color(f: &Fb, rgb: u32) -> u32 {
    let r = (rgb >> 16) & 0xFF;
    let g = (rgb >> 8) & 0xFF;
    let b = rgb & 0xFF;
    let pack = |v: u32, pos: u8, size: u8| -> u32 {
        if size == 0 {
            return 0;
        }
        let max = (1u32 << size) - 1;
        ((v * max + 127) / 255) << pos
    };
    pack(r, f.red_pos, f.red_size) | pack(g, f.green_pos, f.green_size) | pack(b, f.blue_pos, f.blue_size)
}

fn put_pixel(f: &Fb, x: usize, y: usize, rgb: u32) {
    if x >= f.width || y >= f.height {
        return;
    }
    let off = y * f.pitch + x * f.bytes_pp;
    let c = color(f, rgb);
    unsafe {
        let p = (f.addr as *mut u8).add(off);
        if f.bytes_pp == 4 {
            core::ptr::write_volatile(p as *mut u32, c);
        } else {
            core::ptr::write_volatile(p, (c & 0xFF) as u8);
            core::ptr::write_volatile(p.add(1), ((c >> 8) & 0xFF) as u8);
            core::ptr::write_volatile(p.add(2), ((c >> 16) & 0xFF) as u8);
        }
    }
}

fn fill_rect(f: &Fb, x: usize, y: usize, w: usize, h: usize, rgb: u32) {
    let c = color(f, rgb);
    for yy in y..(y + h).min(f.height) {
        let base = yy * f.pitch + x * f.bytes_pp;
        for xx in 0..w.min(f.width.saturating_sub(x)) {
            unsafe {
                let p = (f.addr as *mut u8).add(base + xx * f.bytes_pp);
                if f.bytes_pp == 4 {
                    core::ptr::write_volatile(p as *mut u32, c);
                } else {
                    core::ptr::write_volatile(p, (c & 0xFF) as u8);
                    core::ptr::write_volatile(p.add(1), ((c >> 8) & 0xFF) as u8);
                    core::ptr::write_volatile(p.add(2), ((c >> 16) & 0xFF) as u8);
                }
            }
        }
    }
}

pub fn clear() {
    let f = match unsafe { FB.as_ref() } {
        Some(f) => f,
        None => return,
    };
    fill_rect(f, 0, 0, f.width, f.height, BG);
    unsafe {
        let fb = FB.as_mut().unwrap();
        fb.col = 0;
        fb.row = 0;
    }
}

fn scroll(f: &mut Fb) {
    let line_h = font::GLYPH_H * f.scale;
    let bytes = f.pitch * line_h;
    unsafe {
        core::ptr::copy(
            (f.addr as *const u8).add(bytes),
            f.addr as *mut u8,
            f.pitch * (f.height - line_h),
        );
    }
    fill_rect(f, 0, f.height - line_h, f.width, line_h, BG);
    f.row = f.rows - 1;
}

fn draw_glyph(f: &Fb, ch: char, col: usize, row: usize) {
    let idx = font::glyph_index(ch);
    let glyph = &font::GLYPHS[idx];
    let x0 = col * font::GLYPH_W * f.scale;
    let y0 = row * font::GLYPH_H * f.scale;
    for (gy, bits) in glyph.iter().enumerate() {
        for gx in 0..font::GLYPH_W {
            if *bits & (0x8000u16 >> gx) != 0 {
                for sy in 0..f.scale {
                    for sx in 0..f.scale {
                        put_pixel(
                            f,
                            x0 + gx * f.scale + sx,
                            y0 + gy * f.scale + sy,
                            FG,
                        );
                    }
                }
            }
        }
    }
}

/// Вывести один символ (UTF-8 уже декодирован).
pub fn put_char(ch: char) {
    let f = match unsafe { FB.as_mut() } {
        Some(f) => f,
        None => return,
    };
    match ch {
        '\n' => {
            f.col = 0;
            f.row += 1;
        }
        '\r' => f.col = 0,
        '\t' => {
            f.col = (f.col + 4) & !3;
            if f.col >= f.cols {
                f.col = 0;
                f.row += 1;
            }
        }
        '\u{8}' => {
            if f.col > 0 {
                f.col -= 1;
            }
            fill_rect(
                f,
                f.col * font::GLYPH_W * f.scale,
                f.row * font::GLYPH_H * f.scale,
                font::GLYPH_W * f.scale,
                font::GLYPH_H * f.scale,
                BG,
            );
        }
        _ => {
            draw_glyph(f, ch, f.col, f.row);
            f.col += 1;
            if f.col >= f.cols {
                f.col = 0;
                f.row += 1;
            }
        }
    }
    if f.row >= f.rows {
        scroll(f);
    }
}

pub fn print(s: &str) {
    for ch in s.chars() {
        put_char(ch);
    }
}

pub fn println(s: &str) {
    print(s);
    put_char('\n');
}

/// Диагностика для serial-лога.
pub fn stats() -> (usize, usize, usize, usize) {
    match unsafe { FB.as_ref() } {
        Some(f) => (f.width, f.height, f.cols, f.rows),
        None => (0, 0, 0, 0),
    }
}

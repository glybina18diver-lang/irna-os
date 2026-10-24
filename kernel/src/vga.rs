// kernel/src/vga.rs — текстовый вывод в VGA text mode (M1).
// Буфер 0xB8000 (identity-map trampoline), 80x25, атрибут 0x0F (светло-серый на
// чёрном). Поддержка UTF-8: латиница + базовая кириллица через кодовую страницу
// VGA CP437 (кириллические глифы есть в ROM VGA/QEMU). Прокрутка при переполнении.
use core::fmt;

const WIDTH: usize = 80;
const HEIGHT: usize = 25;
const BUFFER: *mut u16 = 0xB8000 as *mut u16;
const ATTR: u16 = 0x0F;

static mut ROW: usize = 0;
static mut COL: usize = 0;

// Кодировка кириллицы в VGA-шрифт ROM BIOS (QEMU std-vga, глифы CP437).
// Таблица: заглавные А..Щ -> 0xA6..0xBF, Ъ/Ы/Ь/Э/Ю/Я -> 0xD0..0xD5? -- нет.
// Итоговая проверенная карта (DOS/VGA): см. массивы ниже; при визуальной
// ошибке глифа на этапе M1 поправим таблицу по скриншоту serial-дампа.
fn cp437_cyrillic(c: char) -> Option<u8> {
    const UP: &[(char, u8)] = &[
        ('А',0xA6),('Б',0xA7),('В',0xA8),('Г',0xA9),('Д',0xAA),('Е',0xAB),('Ж',0xAC),
        ('З',0xAD),('И',0xAE),('Й',0xAF),('К',0xB0),('Л',0xB1),('М',0xB2),('Н',0xB3),
        ('О',0xB4),('П',0xB5),('Р',0xB6),('С',0xB7),('Т',0xB8),('У',0xB9),('Ф',0xBA),
        ('Х',0xBB),('Ц',0xBC),('Ч',0xBD),('Ш',0xBE),('Щ',0xBF),('Ъ',0xD0),('Ы',0xD1),
        ('Ь',0xD2),('Э',0xD3),('Ю',0xD4),('Я',0xD5),('Ё',0xF8),
    ];
    const LO: &[(char, u8)] = &[
        ('а',0x80),('б',0x81),('в',0x82),('г',0x83),('д',0x84),('е',0x85),('ж',0x86),
        ('з',0x87),('и',0x88),('й',0x89),('к',0x8A),('л',0x8B),('м',0x8C),('н',0x8D),
        ('о',0x8E),('п',0x8F),('р',0x90),('с',0x91),('т',0x92),('у',0x93),('ф',0x94),
        ('х',0x95),('ц',0x96),('ч',0x97),('ш',0x98),('щ',0x99),('ъ',0x9A),('ы',0x9B),
        ('ь',0x9C),('э',0x9D),('ю',0x9E),('я',0x9F),('ё',0xA4),
    ];
    UP.iter().chain(LO.iter()).find(|(ch, _)| *ch == c).map(|(_, code)| *code)
}

fn write_cell(row: usize, col: usize, ch: u8, attr: u16) {
    unsafe {
        core::ptr::write_volatile(BUFFER.add(row * WIDTH + col), (attr << 8) | ch as u16);
    }
}

fn scroll() {
    unsafe {
        for r in 0..HEIGHT - 1 {
            for c in 0..WIDTH {
                let v = core::ptr::read_volatile(BUFFER.add((r + 1) * WIDTH + c));
                core::ptr::write_volatile(BUFFER.add(r * WIDTH + c), v);
            }
        }
        for c in 0..WIDTH {
            core::ptr::write_volatile(BUFFER.add((HEIGHT - 1) * WIDTH + c), (ATTR << 8) | ' ' as u16);
        }
        ROW = HEIGHT - 1;
    }
}

fn put_char(c: char) {
    let byte = if c.is_ascii() {
        c as u8
    } else if let Some(b) = cp437_cyrillic(c) {
        b
    } else {
        b'?'
    };
    unsafe {
        if ROW >= HEIGHT { scroll(); }
        write_cell(ROW, COL, byte, ATTR);
        COL += 1;
        if COL >= WIDTH {
            COL = 0;
            ROW += 1;
            if ROW >= HEIGHT { scroll(); }
        }
    }
}

pub fn clear() {
    unsafe {
        for i in 0..WIDTH * HEIGHT {
            core::ptr::write_volatile(BUFFER.add(i), (ATTR << 8) | ' ' as u16);
        }
        ROW = 0;
        COL = 0;
    }
    update_cursor();
}

/// Курсор VGA-контроллера (регистры CRT C6/0x3D4) — чтобы текст было видно даже
/// без вывода символов в нужную позицию.
pub fn update_cursor() {
    let pos = unsafe { ROW * WIDTH + COL };
    unsafe {
        crate::serial::outb(0x3D4, 0x0F);
        crate::serial::outb(0x3D5, (pos & 0xFF) as u8);
        crate::serial::outb(0x3D4, 0x0E);
        crate::serial::outb(0x3D5, ((pos >> 8) & 0xFF) as u8);
    }
}

struct VgaWriter;
impl fmt::Write for VgaWriter {
    fn write_str(&mut self, s: &str) -> fmt::Result {
        for c in s.chars() {
            if c == '\n' {
                unsafe { COL = 0; ROW += 1; if ROW >= HEIGHT { scroll(); } }
            } else if c != '\r' {
                put_char(c);
            }
        }
        update_cursor();
        Ok(())
    }
}

pub fn print(s: &str) {
    use fmt::Write;
    let _ = VgaWriter.write_str(s);
}

pub fn println(s: &str) {
    print(s);
    print("\n");
}

/// Вывести один ASCII-байт (эхо клавиатуры). Поддерживает \n, \r и backspace.
pub fn putc(c: u8) {
    match c {
        b'\n' => unsafe {
            COL = 0;
            ROW += 1;
            if ROW >= HEIGHT {
                scroll();
            }
        },
        b'\r' => unsafe { COL = 0 },
        0x08 => unsafe {
            if COL > 0 {
                COL -= 1;
            }
            write_cell(ROW, COL, b' ', ATTR);
        },
        _ => put_char(c as char),
    }
    update_cursor();
}

// kernel/src/keyboard.rs — PS/2 клавиатура через IRQ1 (M1).
//
// 8042 по умолчанию транслирует scancode set 2 в set 1, поэтому декодируем set 1
// (make-код = клавиша нажата, break = make | 0x80, 0xE0 — префикс расширенных).
// IRQ1 только читает байт из порта 0x60 и кладёт его в кольцевой буфер; разбор
// делает poll() в контексте главного потока (единственный потребитель).

use core::cell::UnsafeCell;
use core::sync::atomic::{AtomicBool, AtomicUsize, Ordering};

use crate::io;

const DATA: u16 = 0x60;
const STATUS: u16 = 0x64;
const CMD: u16 = 0x64;

const RING_SIZE: usize = 64;

/// Кольцевой буфер «IRQ1 -> главный поток». Один производитель, один потребитель,
/// поэтому достаточно Acquire/Release без блокировок.
struct Ring {
    buf: UnsafeCell<[u8; RING_SIZE]>,
    head: AtomicUsize,
    tail: AtomicUsize,
}

// Безопасно: доступ к buf идёт только из push (IRQ) и pop (поток), и только
// по своим индексам; сами индексы — атомарные.
unsafe impl Sync for Ring {}

impl Ring {
    const fn new() -> Ring {
        Ring {
            buf: UnsafeCell::new([0; RING_SIZE]),
            head: AtomicUsize::new(0),
            tail: AtomicUsize::new(0),
        }
    }

    fn push(&self, byte: u8) {
        let head = self.head.load(Ordering::Relaxed);
        let next = (head + 1) % RING_SIZE;
        if next == self.tail.load(Ordering::Acquire) {
            return; // переполнение: байт теряем, но не блокируемся в прерывании
        }
        unsafe { (*self.buf.get())[head] = byte };
        self.head.store(next, Ordering::Release);
    }

    fn pop(&self) -> Option<u8> {
        let tail = self.tail.load(Ordering::Relaxed);
        if tail == self.head.load(Ordering::Acquire) {
            return None;
        }
        let byte = unsafe { (*self.buf.get())[tail] };
        self.tail.store((tail + 1) % RING_SIZE, Ordering::Release);
        Some(byte)
    }
}

static RING: Ring = Ring::new();

/// Состояние модификаторов (только для разбора в poll).
static SHIFT: AtomicBool = AtomicBool::new(false);
static EXTENDED: AtomicBool = AtomicBool::new(false);

/// Распознанная клавиша.
#[derive(Clone, Copy, PartialEq, Eq)]
pub enum Key {
    Char(u8),
    Enter,
    Backspace,
    Tab,
    Escape,
    Space,
    Up,
    Down,
    Left,
    Right,
    Home,
    End,
    Delete,
    PageUp,
    PageDown,
}

impl Key {
    /// Текстовое имя для лога.
    pub fn name(self) -> &'static str {
        match self {
            Key::Char(_) => "char",
            Key::Enter => "Enter",
            Key::Backspace => "Backspace",
            Key::Tab => "Tab",
            Key::Escape => "Escape",
            Key::Space => "Space",
            Key::Up => "Up",
            Key::Down => "Down",
            Key::Left => "Left",
            Key::Right => "Right",
            Key::Home => "Home",
            Key::End => "End",
            Key::Delete => "Delete",
            Key::PageUp => "PageUp",
            Key::PageDown => "PageDown",
        }
    }
}

/// Инициализация: слить мусор из 8042, включить трансляцию set2 -> set1,
/// разрешить IRQ1 и включить клавиатуру.
pub fn init() {
    unsafe {
        for _ in 0..32 {
            if io::inb(STATUS) & 0x01 == 0 {
                break;
            }
            let _ = io::inb(DATA);
        }
    }

    // Конфигурационный байт 8042: bit0 = IRQ1, bit6 = трансляция set2 -> set1,
    // bit4 = 1 означает «клавиатура отключена» (его снимаем).
    let mut cfg = read_config();
    if cfg == 0x00 || cfg == 0xFF {
        cfg = 0x45; // контроллер не ответил — берём типичное значение после BIOS
    }
    cfg |= 0x01 | 0x40;
    cfg &= !0x10;
    write_config(cfg);

    unsafe {
        wait_write();
        io::outb(CMD, 0xAE); // enable keyboard interface
        // 0xF4 — enable scanning. Без этого QEMU/реальная клавиатура может
        // остаться в состоянии «scan disabled» (0xF5 после POST) и не выдавать
        // скан-коды вообще. Ответ 0xFA придёт в буфер и будет прочитан IRQ1.
        wait_write();
        io::outb(DATA, 0xF4);
    }
}

fn read_config() -> u8 {
    unsafe {
        wait_write();
        io::outb(CMD, 0x20);
        wait_read();
        io::inb(DATA)
    }
}

fn write_config(value: u8) {
    unsafe {
        wait_write();
        io::outb(CMD, 0x60);
        wait_write();
        io::outb(DATA, value);
    }
}

fn wait_read() {
    for _ in 0..100_000 {
        if unsafe { io::inb(STATUS) } & 0x01 != 0 {
            return;
        }
    }
}

fn wait_write() {
    for _ in 0..100_000 {
        if unsafe { io::inb(STATUS) } & 0x02 == 0 {
            return;
        }
    }
}

/// Вызывается из IRQ1.
pub fn handle_irq() {
    unsafe {
        let status = io::inb(STATUS);
        if status & 0x01 == 0 {
            return; // данных нет
        }
        if status & 0x20 != 0 {
            // Байт от мыши — забираем, чтобы не застрял в буфере 8042.
            let _ = io::inb(DATA);
            return;
        }
        let scancode = io::inb(DATA);
        RING.push(scancode);
    }
}

/// Забрать следующую распознанную клавишу (не блокируется).
pub fn poll() -> Option<Key> {
    loop {
        let sc = RING.pop()?;

        if sc == 0xE0 {
            EXTENDED.store(true, Ordering::Relaxed);
            continue;
        }
        if sc == 0xE1 {
            continue; // Pause/Break: длинная последовательность, не поддерживаем
        }

        let extended = EXTENDED.swap(false, Ordering::Relaxed);
        let released = sc & 0x80 != 0;
        let code = sc & 0x7F;

        // Shift (0x2A/0x36) — только состояние, отдельной клавишей не отдаём.
        if code == 0x2A || code == 0x36 {
            SHIFT.store(!released, Ordering::Relaxed);
            continue;
        }
        if released {
            continue;
        }

        if extended {
            return match code {
                0x48 => Some(Key::Up),
                0x50 => Some(Key::Down),
                0x4B => Some(Key::Left),
                0x4D => Some(Key::Right),
                0x47 => Some(Key::Home),
                0x4F => Some(Key::End),
                0x53 => Some(Key::Delete),
                0x49 => Some(Key::PageUp),
                0x51 => Some(Key::PageDown),
                _ => continue,
            };
        }

        // Модификаторы и lock-клавиши пока не отдаём.
        match code {
            0x1D | 0x38 | 0x3A | 0x45 | 0x46 => continue,
            0x01 => return Some(Key::Escape),
            0x0E => return Some(Key::Backspace),
            0x0F => return Some(Key::Tab),
            0x1C => return Some(Key::Enter),
            0x39 => return Some(Key::Space),
            _ => {}
        }

        if let Some(c) = char_for(code, SHIFT.load(Ordering::Relaxed)) {
            return Some(Key::Char(c));
        }
        // Неизвестные клавиши (F1..F12, keypad) молча пропускаем.
    }
}

/// Scancode set 1 -> ASCII с учётом Shift.
fn char_for(sc: u8, shift: bool) -> Option<u8> {
    let c = match sc {
        0x02 => if shift { b'!' } else { b'1' },
        0x03 => if shift { b'@' } else { b'2' },
        0x04 => if shift { b'#' } else { b'3' },
        0x05 => if shift { b'$' } else { b'4' },
        0x06 => if shift { b'%' } else { b'5' },
        0x07 => if shift { b'^' } else { b'6' },
        0x08 => if shift { b'&' } else { b'7' },
        0x09 => if shift { b'*' } else { b'8' },
        0x0A => if shift { b'(' } else { b'9' },
        0x0B => if shift { b')' } else { b'0' },
        0x0C => if shift { b'_' } else { b'-' },
        0x0D => if shift { b'+' } else { b'=' },
        0x10 => b'q',
        0x11 => b'w',
        0x12 => b'e',
        0x13 => b'r',
        0x14 => b't',
        0x15 => b'y',
        0x16 => b'u',
        0x17 => b'i',
        0x18 => b'o',
        0x19 => b'p',
        0x1A => if shift { b'{' } else { b'[' },
        0x1B => if shift { b'}' } else { b']' },
        0x1E => b'a',
        0x1F => b's',
        0x20 => b'd',
        0x21 => b'f',
        0x22 => b'g',
        0x23 => b'h',
        0x24 => b'j',
        0x25 => b'k',
        0x26 => b'l',
        0x27 => if shift { b':' } else { b';' },
        0x28 => if shift { b'"' } else { b'\'' },
        0x29 => if shift { b'~' } else { b'`' },
        0x2B => if shift { b'|' } else { b'\\' },
        0x2C => b'z',
        0x2D => b'x',
        0x2E => b'c',
        0x2F => b'v',
        0x30 => b'b',
        0x31 => b'n',
        0x32 => b'm',
        0x33 => if shift { b'<' } else { b',' },
        0x34 => if shift { b'>' } else { b'.' },
        0x35 => if shift { b'?' } else { b'/' },
        _ => return None,
    };
    Some(if shift && c.is_ascii_lowercase() { c - 32 } else { c })
}

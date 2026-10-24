// kernel/src/pit.rs — программируемый таймер 8253/8254, канал 0 (M1).
// Канал 0 генерирует IRQ0 с частотой PIT_HZ. Делитель = 1 193 182 / PIT_HZ.
// Счётчик тиков увеличивается в обработчике IRQ0 (irq.rs).

use core::sync::atomic::{AtomicU64, Ordering};

use crate::io::outb;

pub const PIT_HZ: u32 = 100;

const PIT_INPUT_HZ: u32 = 1_193_182;
const PIT_CH0: u16 = 0x40;
const PIT_CMD: u16 = 0x43;
/// Канал 0, доступ lo/hi, режим 3 (square wave).
const PIT_MODE3: u8 = 0x36;

static TICKS: AtomicU64 = AtomicU64::new(0);

/// Запрограммировать канал 0 на PIT_HZ.
pub fn init() {
    let divisor = (PIT_INPUT_HZ / PIT_HZ) as u16; // 11931 -> 100.007 Гц
    unsafe {
        outb(PIT_CMD, PIT_MODE3);
        outb(PIT_CH0, (divisor & 0xFF) as u8);
        outb(PIT_CH0, (divisor >> 8) as u8);
    }
}

/// Вызывается из обработчика IRQ0.
pub fn tick() {
    TICKS.fetch_add(1, Ordering::Relaxed);
}

/// Сколько тиков прошло с загрузки.
pub fn ticks() -> u64 {
    TICKS.load(Ordering::Relaxed)
}

/// Подождать `n` тиков (засыпает в hlt до прихода IRQ0).
pub fn wait_ticks(n: u64) {
    let target = ticks() + n;
    while ticks() < target {
        unsafe { core::arch::asm!("hlt", options(nomem, nostack)) };
    }
}

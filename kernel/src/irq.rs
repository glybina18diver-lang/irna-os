// kernel/src/irq.rs — диспетчер аппаратных прерываний (M1).
// Вызывается из общего stub'а (arch.asm) для векторов 0x20..0x2F.
// Обрабатывает ложные IRQ7/IRQ15 (spurious), считает неожиданные линии.

use core::sync::atomic::{AtomicU64, Ordering};

use crate::idt::InterruptFrame;
use crate::{pic, pit};

/// Сколько раз пришло прерывание, для которого нет обработчика.
static UNEXPECTED: AtomicU64 = AtomicU64::new(0);

pub fn dispatch(frame: &InterruptFrame) {
    let irq = (frame.vector - pic::IRQ_BASE as u64) as u8;

    // Ложное IRQ7: master не поднимал линию — EOI не отправляем вообще.
    if irq == 7 && (pic::read_isr(true) & 0x80) == 0 {
        UNEXPECTED.fetch_add(1, Ordering::Relaxed);
        return;
    }
    // Ложное IRQ15: slave линию не поднимал — EOI только master.
    if irq == 15 && (pic::read_isr(false) & 0x80) == 0 {
        UNEXPECTED.fetch_add(1, Ordering::Relaxed);
        pic::eoi_master_only();
        return;
    }

    match irq {
        0 => pit::tick(), // IRQ0 — таймер
        _ => {
            UNEXPECTED.fetch_add(1, Ordering::Relaxed);
        }
    }

    pic::eoi(irq);
}

/// Счётчик необработанных/ложных прерываний (для диагностики).
pub fn unexpected() -> u64 {
    UNEXPECTED.load(Ordering::Relaxed)
}

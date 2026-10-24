// kernel/src/pic.rs — контроллер прерываний 8259A (M1).
// BIOS отображает IRQ0..7 на векторы 0x08..0x0F, что пересекается с исключениями
// процессора (#DF = 8, #GP = 13). Перепрошиваем PIC на 0x20..0x2F:
//   master -> 0x20..0x27 (IRQ0..7), slave -> 0x28..0x2F (IRQ8..15).
// Каскад: slave подключён к IRQ2 master.

use crate::io::{inb, io_wait, outb};

pub const IRQ_BASE: u8 = 0x20;
pub const IRQ_COUNT: u8 = 16;

const PIC1_CMD: u16 = 0x20;
const PIC1_DATA: u16 = 0x21;
const PIC2_CMD: u16 = 0xA0;
const PIC2_DATA: u16 = 0xA1;

const ICW1_INIT: u8 = 0x11; // инициализация + потребуется ICW4
const ICW4_8086: u8 = 0x01; // режим 8086/88

const OCW2_EOI: u8 = 0x20;
const OCW3_READ_ISR: u8 = 0x0B;

/// Перепрошить PIC и замаскировать все IRQ, кроме явно разрешённых.
pub fn init() {
    unsafe {
        // ICW1: старт инициализации, ICW4 будет.
        outb(PIC1_CMD, ICW1_INIT);
        io_wait();
        outb(PIC2_CMD, ICW1_INIT);
        io_wait();

        // ICW2: новые базовые векторы.
        outb(PIC1_DATA, IRQ_BASE);
        io_wait();
        outb(PIC2_DATA, IRQ_BASE + 8);
        io_wait();

        // ICW3: master — slave на линии IRQ2 (бит 2); slave — каскад 2.
        outb(PIC1_DATA, 0x04);
        io_wait();
        outb(PIC2_DATA, 0x02);
        io_wait();

        // ICW4: режим 8086.
        outb(PIC1_DATA, ICW4_8086);
        io_wait();
        outb(PIC2_DATA, ICW4_8086);
        io_wait();

        // Пока все линии замаскированы; конкретные IRQ разрешает подсистема.
        outb(PIC1_DATA, 0xFF);
        outb(PIC2_DATA, 0xFF);
    }
}

/// Разрешить линию IRQ (0..15).
pub fn unmask(irq: u8) {
    let (port, bit) = if irq < 8 {
        (PIC1_DATA, irq)
    } else {
        (PIC2_DATA, irq - 8)
    };
    unsafe {
        let mask = inb(port);
        outb(port, mask & !(1 << bit));
        if irq >= 8 {
            // slave подключён к IRQ2 master — линию каскада тоже открываем.
            let m = inb(PIC1_DATA);
            outb(PIC1_DATA, m & !(1 << 2));
        }
    }
}

/// Замаскировать линию IRQ (0..15).
pub fn mask(irq: u8) {
    let (port, bit) = if irq < 8 {
        (PIC1_DATA, irq)
    } else {
        (PIC2_DATA, irq - 8)
    };
    unsafe {
        let mask = inb(port);
        outb(port, mask | (1 << bit));
    }
}

/// Подтвердить прерывание (EOI). Для slave — сначала slave, потом master.
pub fn eoi(irq: u8) {
    unsafe {
        if irq >= 8 {
            outb(PIC2_CMD, OCW2_EOI);
        }
        outb(PIC1_CMD, OCW2_EOI);
    }
}

/// Только master-EOI — нужен для ложного IRQ15 (slave не поднимал линию).
pub fn eoi_master_only() {
    unsafe { outb(PIC1_CMD, OCW2_EOI) };
}

/// Прочитать текущую маску (1 = линия замаскирована).
#[allow(dead_code)] // используется при отладке прерываний
pub fn read_mask(master: bool) -> u8 {
    unsafe { inb(if master { PIC1_DATA } else { PIC2_DATA }) }
}

/// Прочитать регистр ISR (какие линии реально запрошены) — для детекта
/// ложных прерываний IRQ7/IRQ15.
pub fn read_isr(master: bool) -> u8 {
    let cmd = if master { PIC1_CMD } else { PIC2_CMD };
    unsafe {
        outb(cmd, OCW3_READ_ISR);
        inb(cmd)
    }
}

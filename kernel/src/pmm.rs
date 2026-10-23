// kernel/src/pmm.rs — физический аллокатор кадров (M1).
// Bitmap-аллокатор: 1 бит на 4 KiB-кадр. Таблица размещается в статическом
// буфере, покрытом identity-отображением trampoline (адрес < 1 GiB).
use crate::bootinfo;

pub const FRAME_SIZE: u64 = 4096;
/// Максимум отслеживаемой памяти: 512 МиБ => 131072 кадра => 16 KiB bitmap.
const MAX_FRAMES: usize = 128 * 1024;
static mut BITMAP: [u64; MAX_FRAMES / 64] = [0; MAX_FRAMES / 64];
static mut TOTAL_FRAMES: usize = 0;
static mut FREE_FRAMES: usize = 0;

#[inline]
unsafe fn get_bit(i: usize) -> bool {
    BITMAP[i / 64] & (1 << (i % 64)) != 0
}
#[inline]
unsafe fn set_bit(i: usize) {
    BITMAP[i / 64] |= 1 << (i % 64);
}
#[inline]
unsafe fn clear_bit(i: usize) {
    BITMAP[i / 64] &= !(1 << (i % 64));
}

unsafe extern "Rust" {
    static _end: u8; // метка linker.ld: конец загруженного образа ядра
}

/// Инициализация по memory map загрузчика. Кадр [0.._end) и область ниже 1MiB
/// помечаются занятыми; всё остальное usable — свободно.
pub unsafe fn init(info: &bootinfo::BootInfo) {
    let top = if info.free_mem_top >= 0x0800_0000 {
        0x0800_0000u64 // не выходим за пределы identity-карты trampoline
    } else if info.free_mem_top > 0 {
        info.free_mem_top
    } else {
        0x0800_0000 // fallback, если MBI не разобрана
    };
    TOTAL_FRAMES = (top / FRAME_SIZE) as usize;
    if TOTAL_FRAMES > MAX_FRAMES {
        TOTAL_FRAMES = MAX_FRAMES;
    }
    // помечаем весь диапазон занятым, затем освобождаем выше ядра
    for i in 0..TOTAL_FRAMES {
        set_bit(i);
    }
    FREE_FRAMES = 0;
    let kernel_end = (&raw const _end) as u64;
    let first_free = ((kernel_end + FRAME_SIZE - 1) / FRAME_SIZE) * FRAME_SIZE;
    let mut f = (first_free / FRAME_SIZE) as usize;
    while f < TOTAL_FRAMES {
        clear_bit(f);
        FREE_FRAMES += 1;
        f += 1;
    }
}

/// Выделить один кадр (4 KiB), вернуть физический==линейный адрес или None.
pub unsafe fn alloc_frame() -> Option<u64> {
    // поиск первого нулевого бита со сканированием слов
    for w in 0..(TOTAL_FRAMES / 64) {
        if BITMAP[w] == u64::MAX {
            continue;
        }
        let bit = (!BITMAP[w]).trailing_zeros() as usize;
        let idx = w * 64 + bit;
        if idx < TOTAL_FRAMES {
            set_bit(idx);
            FREE_FRAMES -= 1;
            return Some(idx as u64 * FRAME_SIZE);
        }
    }
    None
}

pub unsafe fn free_frame(addr: u64) {
    let idx = (addr / FRAME_SIZE) as usize;
    if idx < TOTAL_FRAMES && get_bit(idx) {
        clear_bit(idx);
        FREE_FRAMES += 1;
    }
}

pub unsafe fn stats() -> (usize, usize) {
    (FREE_FRAMES, TOTAL_FRAMES)
}

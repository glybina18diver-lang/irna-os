// kernel/src/heap.rs — куча ядра (M1): статический пул 1 МиБ + bump-аллокатор.
// Сознательное упрощение этапа: next_power_of_two, выравнивание до 64, без
// освобождения (dealloc no-op). Полноценный slab/buddy — на M2 вместе с VMA.
use core::alloc::{GlobalAlloc, Layout};
use core::sync::atomic::{AtomicUsize, Ordering};

const POOL_SIZE: usize = 1024 * 1024; // 1 MiB

#[repr(C, align(16))]
struct Pool([u8; POOL_SIZE]);
static mut POOL: Pool = Pool([0; POOL_SIZE]);
/// Индекс следующего свободного блока в «линейном» резерве (simple bump).
static NEXT: AtomicUsize = AtomicUsize::new(0);

pub struct KernelHeap;

unsafe impl GlobalAlloc for KernelHeap {
    unsafe fn alloc(&self, layout: Layout) -> *mut u8 {
        // Выравнивание+размер округляем до степени двойки >= max(size,32), align<=64
        let size = layout.size().max(32);
        let sz = size.next_power_of_two();
        if sz > POOL_SIZE || layout.align() > 64 {
            return core::ptr::null_mut();
        }
        let align = layout.align().max(16);
        loop {
            let cur = NEXT.load(Ordering::Relaxed);
            let start = (cur + align - 1) & !(align - 1);
            if start + sz > POOL_SIZE {
                return core::ptr::null_mut(); // куча исчерпана
            }
            if NEXT
                .compare_exchange_weak(cur, start + sz, Ordering::AcqRel, Ordering::Relaxed)
                .is_ok()
            {
                return (core::ptr::addr_of_mut!(POOL) as *mut u8).add(start);
            }
        }
    }

    unsafe fn dealloc(&self, _ptr: *mut u8, _layout: Layout) {
        // M1: освобождение не выполняется (ядру хватит 1 МиБ на этап; очистка
        // при выгрузке модулей появится вместе с VMA на M2).
    }
}

#[global_allocator]
pub static ALLOCATOR: KernelHeap = KernelHeap;

/// Занято байт из пула (для диагностики в логе загрузки).
pub fn used() -> usize {
    NEXT.load(Ordering::Relaxed)
}
pub const CAPACITY: usize = POOL_SIZE;

// kernel/src/paging.rs — минимальное управление page tables (M1, framebuffer).
//
// Trampoline (arch.asm) строит identity-map 0..1 ГиБ: pml4[0] -> pdpt,
// pdpt[0] -> pd_lo (0..512 МиБ, RW), pdpt[1] -> pd_hi (512 МиБ..1 ГиБ, MMIO RO).
// Линейный framebuffer (LFB) обычно лежит выше 1 ГиБ (у QEMU — 0xFD000000),
// поэтому его нужно отобразить отдельно.
//
// Здесь — только то, что нужно M1: отображение физического диапазона
// identity-адресом страницами по 2 МиБ с флагом RW. Page tables берутся из
// статического пула (8 таблиц = 32 КиБ в .bss), PML4/PDPT — из arch.asm.

const P: u64 = 1 << 0; // present
const RW: u64 = 1 << 1; // writable
const US: u64 = 1 << 2; // user-accessible
const PS: u64 = 1 << 7; // page size (2 МиБ в PD)

const PAGE_2M: u64 = 2 * 1024 * 1024;
const PT_ENTRIES: usize = 512;
const PT_POOL_COUNT: usize = 8;

#[repr(C, align(4096))]
struct PageTable([u64; PT_ENTRIES]);

static mut PT_POOL: [PageTable; PT_POOL_COUNT] = [const { PageTable([0; PT_ENTRIES]) }; PT_POOL_COUNT];
static mut PT_USED: usize = 0;

extern "C" {
    static pml4: u8;
    static pdpt: u8;
}

fn pml4_ptr() -> *mut u64 {
    unsafe { &pml4 as *const u8 as *mut u64 }
}

fn pdpt_ptr() -> *mut u64 {
    unsafe { &pdpt as *const u8 as *mut u64 }
}

/// Выделить нулевую page table из статического пула.
fn alloc_pt() -> Option<*mut u64> {
    unsafe {
        if PT_USED >= PT_POOL_COUNT {
            return None;
        }
        let p = core::ptr::addr_of_mut!(PT_POOL) as *mut PageTable;
        let pt = p.add(PT_USED);
        PT_USED += 1;
        (*pt).0 = [0; PT_ENTRIES];
        Some((*pt).0.as_mut_ptr())
    }
}

/// Отобразить физический диапазон `[phys, phys+len)` по тем же адресам (identity)
/// страницами 2 МиБ с флагом RW. Возвращает false, если не хватило page tables
/// или адрес вне поддерживаемого pml4[0] (0..512 ГиБ).
pub fn map_identity_2m(phys: u64, len: u64) -> bool {
    if len == 0 {
        return true;
    }
    let start = phys & !(PAGE_2M - 1);
    let end = (phys + len + PAGE_2M - 1) & !(PAGE_2M - 1);

    let mut addr = start;
    while addr < end {
        if (addr >> 39) & 0x1FF != 0 {
            return false; // вне pml4[0]
        }
        let pdpt_idx = ((addr >> 30) & 0x1FF) as usize;
        let pd_idx = ((addr >> 21) & 0x1FF) as usize;

        unsafe {
            let pdpt_tbl = pdpt_ptr();
            let mut entry = core::ptr::read_volatile(pdpt_tbl.add(pdpt_idx));
            let pd: *mut u64;
            if entry & P == 0 {
                // нет PD для этого ГиБ — выделяем
                match alloc_pt() {
                    Some(p) => {
                        pd = p;
                        entry = (p as u64) | P | RW;
                        core::ptr::write_volatile(pdpt_tbl.add(pdpt_idx), entry);
                    }
                    None => return false,
                }
            } else if entry & PS != 0 {
                // уже отображено страницей 1 ГиБ — просто разрешаем запись
                core::ptr::write_volatile(pdpt_tbl.add(pdpt_idx), entry | RW);
                addr += PAGE_2M;
                continue;
            } else {
                pd = (entry & !0xFFF) as *mut u64;
            }

            let want = addr | P | RW | PS;
            let cur = core::ptr::read_volatile(pd.add(pd_idx));
            if cur & P == 0 {
                core::ptr::write_volatile(pd.add(pd_idx), want);
            } else if cur & PS != 0 {
                core::ptr::write_volatile(pd.add(pd_idx), cur | RW);
            }
        }
        addr += PAGE_2M;
    }

    // Сбросить TLB: перезагрузка CR3 (identity-map не меняется).
    flush_tlb();
    true
}

/// Сбросить TLB перезагрузкой CR3 (содержимое таблиц не меняется).
fn flush_tlb() {
    unsafe {
        let cr3: u64;
        core::arch::asm!("mov {}, cr3", out(reg) cr3, options(nostack, preserves_flags));
        core::arch::asm!("mov cr3, {}", in(reg) cr3, options(nostack, preserves_flags));
    }
}

/// Отобразить 4-КиБ страницу как доступную из ring 3 (`U/S=1`).
///
/// Если соответствующая запись PD — huge-страница 2 МиБ (так устроена карта
/// trampoline'а), она расщепляется: выделяется PT, заполняется identity-
/// отображением того же диапазона, после чего нужная запись заменяется.
/// Возвращает false, если таблиц не хватило или адрес вне pml4[0].
pub fn map_user_4k(virt: u64, phys: u64, writable: bool) -> bool {
    if virt & 0xFFF != 0 || phys & 0xFFF != 0 {
        return false;
    }
    if (virt >> 39) & 0x1FF != 0 {
        return false;
    }
    let pdpt_idx = ((virt >> 30) & 0x1FF) as usize;
    let pd_idx = ((virt >> 21) & 0x1FF) as usize;
    let pt_idx = ((virt >> 12) & 0x1FF) as usize;

    unsafe {
        let pdpt_tbl = pdpt_ptr();
        let pml4e = core::ptr::read_volatile(pml4_ptr());
        if pml4e & P == 0 {
            return false;
        }
        // U/S проверяется на КАЖДОМ уровне: без него в PML4/PDPT/PD доступ из
        // ring 3 даёт #PF (protection violation), даже если PT-запись с U/S.
        core::ptr::write_volatile(pml4_ptr(), pml4e | US);

        let pde = core::ptr::read_volatile(pdpt_tbl.add(pdpt_idx));
        if pde & P == 0 || pde & PS != 0 {
            return false; // нет PD или 1-ГиБ huge: в M1 не поддерживаем
        }
        core::ptr::write_volatile(pdpt_tbl.add(pdpt_idx), pde | US);
        let pd = (pde & !0xFFF) as *mut u64;
        let entry = core::ptr::read_volatile(pd.add(pd_idx));
        if entry & P == 0 {
            return false;
        }

        let pt: *mut u64;
        if entry & PS != 0 {
            match alloc_pt() {
                Some(p) => pt = p,
                None => return false,
            }
            let base = entry & !(PAGE_2M - 1);
            for i in 0..PT_ENTRIES {
                let pa = base + (i as u64) * 4096;
                core::ptr::write_volatile(pt.add(i), pa | P | RW);
            }
            // PD ссылается на PT: RW наследуется, U/S добавляется — иначе
            // пользователь не пройдёт проверку на уровне PD.
            let flags = (entry & RW) | US;
            core::ptr::write_volatile(pd.add(pd_idx), (pt as u64) | P | flags);
        } else {
            pt = (entry & !0xFFF) as *mut u64;
            core::ptr::write_volatile(pd.add(pd_idx), entry | US);
        }

        let mut flags = P | US;
        if writable {
            flags |= RW;
        }
        core::ptr::write_volatile(pt.add(pt_idx), (phys & !0xFFF) | flags);
    }
    flush_tlb();
    true
}

/// Сколько page tables уже израсходовано (диагностика).
pub fn used_tables() -> usize {
    unsafe { PT_USED }
}

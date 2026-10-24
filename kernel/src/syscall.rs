// kernel/src/syscall.rs — SYSCALL/SYSRET (M1): настройка MSR и диспетчер.
//
// Соглашение — Linux-подобное: номер в RAX, аргументы в RDI, RSI, RDX, R10,
// R8, R9, результат в RAX (отрицательное значение — -errno). Точка входа —
// syscall_entry в arch.asm: она переключает стек, сохраняет контекст и зовёт
// syscall_dispatch.
//
// M1 реализует минимум: write (fd 1/2) и exit. read пока возвращает -ENOSYS —
// блокирующее чтение появится вместе с планировщиком в M2.

use crate::{console, serial};

const MSR_EFER: u32 = 0xC000_0080;
const MSR_STAR: u32 = 0xC000_0081;
const MSR_LSTAR: u32 = 0xC000_0082;
const MSR_FMASK: u32 = 0xC000_0084;

const EFER_SCE: u64 = 1 << 0;

pub const SYS_READ: u64 = 0;
pub const SYS_WRITE: u64 = 1;
pub const SYS_EXIT: u64 = 60;

const EBADF: u64 = 9;
const EFAULT: u64 = 14;
const ENOSYS: u64 = 38;

extern "C" {
    fn syscall_entry();
    fn syscall_exit_to_kernel() -> !;
}

#[inline]
unsafe fn rdmsr(msr: u32) -> u64 {
    let lo: u32;
    let hi: u32;
    core::arch::asm!("rdmsr", in("ecx") msr, out("eax") lo, out("edx") hi, options(nomem, nostack));
    ((hi as u64) << 32) | lo as u64
}

#[inline]
unsafe fn wrmsr(msr: u32, val: u64) {
    core::arch::asm!(
        "wrmsr",
        in("ecx") msr,
        in("eax") val as u32,
        in("edx") (val >> 32) as u32,
        options(nomem, nostack)
    );
}

/// Включить SYSCALL/SYSRET и настроить сегменты/маску флагов.
///
/// STAR[47:32] = 0x08 → на входе CS=0x08, SS=0x10 (ядро).
/// STAR[63:48] = 0x13 → на выходе CS=0x23, SS=0x1B (ring 3; см. gdt.rs).
/// FMASK сбрасывает TF/IF/DF/NT/AC: обработчик работает с запрещёнными
/// прерываниями, а R11 (сохранённые флаги) восстанавливает их на sysret.
pub unsafe fn init() {
    wrmsr(MSR_EFER, rdmsr(MSR_EFER) | EFER_SCE);
    wrmsr(MSR_STAR, (0x13u64 << 48) | (0x08u64 << 32));
    wrmsr(MSR_LSTAR, syscall_entry as u64);
    wrmsr(MSR_FMASK, 0x47700);
}

/// Диспетчер syscall (вызывается из arch.asm).
#[unsafe(no_mangle)]
pub extern "C" fn syscall_dispatch(num: u64, a1: u64, a2: u64, a3: u64, _a4: u64, _a5: u64) -> u64 {
    match num {
        SYS_WRITE => sys_write(a1, a2, a3),
        SYS_READ => err(ENOSYS),
        SYS_EXIT => {
            serial::write_str("sys_exit(");
            serial::write_dec_u64(a1);
            serial::write_str("): возврат в ядро\n");
            console::println("sys_exit: возврат в ядро из ring 3");
            unsafe { syscall_exit_to_kernel() }
        }
        _ => {
            serial::write_str("syscall: неизвестный номер ");
            serial::write_dec_u64(num);
            serial::write_str("\n");
            err(ENOSYS)
        }
    }
}

#[inline]
fn err(code: u64) -> u64 {
    (code as i64).wrapping_neg() as u64
}

/// write(fd, buf, len): fd 1/2 — serial + консоль. Буфер — адрес в текущем
/// адресном пространстве (в M1 пользовательские страницы видны ядру напрямую).
fn sys_write(fd: u64, buf: u64, len: u64) -> u64 {
    if fd != 1 && fd != 2 {
        return err(EBADF);
    }
    if len == 0 {
        return 0;
    }
    if buf == 0 || len > 4096 {
        return err(EFAULT);
    }
    let bytes = unsafe { core::slice::from_raw_parts(buf as *const u8, len as usize) };
    serial::write_bytes(bytes);
    match core::str::from_utf8(bytes) {
        Ok(s) => console::print(s),
        Err(_) => {
            for &b in bytes {
                console::putc(b);
            }
        }
    }
    len
}

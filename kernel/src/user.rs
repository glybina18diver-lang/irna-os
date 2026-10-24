// kernel/src/user.rs — smoke-тест ring 3 (M1): iretq в user mode, syscall,
// возврат в ядро через sys_exit.
//
// Тест намеренно минимальный: одна страница кода и одна страница стека,
// отображённые с U/S=1. Код вызывает write(1, msg, len) и exit(0). Это
// проверяет всю цепочку: GDT-селекторы ring 3, TSS.RSP0, IRETQ, SYSCALL
// (LSTAR/STAR/FMASK), диспетчер и возврат в ядро.
//
// Адреса выбраны в свободном диапазоне 4..6 МиБ: там лежит huge-страница
// pd_lo[2], которую paging::map_user_4k расщепляет на 4-КиБ страницы.

use crate::{console, paging, pmm, serial};

const CODE_VADDR: u64 = 0x40_0000;
const STACK_VADDR: u64 = 0x40_2000;
const MSG_OFF: usize = 0x40;
const MSG: &str = "Привет из ring 3! syscall/sysret OK\n";

extern "C" {
    fn enter_user_mode(rip: u64, rsp: u64) -> !;
}

fn halt() -> ! {
    loop {
        unsafe { core::arch::asm!("cli", "hlt") };
    }
}

/// Собрать и запустить пользовательскую программу. Не возвращается:
/// после sys_exit управление уходит в kernel_after_user (arch.asm).
pub fn run_test() -> ! {
    let code_phys = match unsafe { pmm::alloc_frame() } {
        Some(p) => p,
        None => {
            serial::write_str("user: нет свободного кадра под код\n");
            halt();
        }
    };
    let stack_phys = match unsafe { pmm::alloc_frame() } {
        Some(p) => p,
        None => {
            serial::write_str("user: нет свободного кадра под стек\n");
            halt();
        }
    };

    // Сначала RW — ядру нужно записать код и сообщение.
    if !paging::map_user_4k(CODE_VADDR, code_phys, true)
        || !paging::map_user_4k(STACK_VADDR, stack_phys, true)
    {
        serial::write_str("user: map_user_4k FAILED\n");
        halt();
    }

    // mov eax,1 / mov edi,1 / mov esi,msg / mov edx,len / syscall /
    // mov eax,60 / xor edi,edi / syscall / jmp $
    let code: [u8; 33] = [
        0xB8, 0x01, 0x00, 0x00, 0x00, 0xBF, 0x01, 0x00, 0x00, 0x00, 0xBE, 0x00, 0x00, 0x00, 0x00,
        0xBA, 0x00, 0x00, 0x00, 0x00, 0x0F, 0x05, 0xB8, 0x3C, 0x00, 0x00, 0x00, 0x31, 0xFF, 0x0F,
        0x05, 0xEB, 0xFE,
    ];
    let msg_addr = (CODE_VADDR + MSG_OFF as u64) as u32;
    let msg_len = MSG.len() as u32;

    unsafe {
        let p = CODE_VADDR as *mut u8;
        core::ptr::copy_nonoverlapping(code.as_ptr(), p, code.len());
        core::ptr::copy_nonoverlapping(msg_addr.to_le_bytes().as_ptr(), p.add(11), 4);
        core::ptr::copy_nonoverlapping(msg_len.to_le_bytes().as_ptr(), p.add(16), 4);
        core::ptr::copy_nonoverlapping(MSG.as_ptr(), p.add(MSG_OFF), MSG.len());
    }

    // Код готов — снимаем право записи (CR0.WP=1, поэтому ядро тоже не запишет).
    if !paging::map_user_4k(CODE_VADDR, code_phys, false) {
        serial::write_str("user: защита кода (RO) FAILED\n");
        halt();
    }

    serial::write_str("ring 3: iretq в user mode, код по адресу ");
    serial::write_hex_u64(CODE_VADDR);
    serial::write_str(", стек ");
    serial::write_hex_u64(STACK_VADDR + 4096);
    serial::write_str("\n");

    unsafe { enter_user_mode(CODE_VADDR, STACK_VADDR + 4096 - 16) }
}

/// Продолжение работы ядра после sys_exit (вызывается из arch.asm).
#[unsafe(no_mangle)]
pub extern "C" fn kernel_after_user() -> ! {
    serial::write_str("ring 3: syscall/sysret round-trip OK\n");
    console::println("ring 3: syscall/sysret round-trip OK");
    crate::echo_loop()
}

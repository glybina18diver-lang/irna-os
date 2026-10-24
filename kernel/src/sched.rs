// kernel/src/sched.rs — кооперативный планировщик (M1).
//
// Модель: N задач в статическом массиве, у каждой свой стек 16 КиБ. Задача
// отдаёт процессор явно (yield_now) или завершается (task_exit). Главный поток
// (main) выступает «диспетчером»: run() выбирает следующую готовую задачу,
// переключается в неё, а по yield/exit возвращается обратно в run().
//
// Переключение — switch_context в arch.asm: сохраняет callee-saved регистры
// (SysV) и меняет RSP. Это классическая схема «стек на задачу», которая
// позже станет базой для вытесняющего планировщика (по PIT) и для процессов
// с отдельными адресными пространствами (M2).
//
// Ограничения M1: одно ядро, нет приоритетов, нет вытеснения, нет
// блокирующих примитивов. Всё это — следующий шаг.

const MAX_TASKS: usize = 8;
const STACK_SIZE: usize = 16 * 1024;

#[derive(Clone, Copy, PartialEq)]
enum State {
    Free,
    Ready,
    Dead,
}

#[derive(Clone, Copy)]
struct Task {
    rsp: u64,
    state: State,
}

#[repr(C, align(16))]
#[allow(dead_code)]
struct Stack([u8; STACK_SIZE]);

static mut TASKS: [Task; MAX_TASKS] = [Task {
    rsp: 0,
    state: State::Free,
}; MAX_TASKS];
static mut STACKS: [Stack; MAX_TASKS] = [const { Stack([0; STACK_SIZE]) }; MAX_TASKS];

/// RSP диспетчера (main) — точка возврата из задачи.
static mut MAIN_RSP: u64 = 0;
/// Индекс текущей задачи; MAX_TASKS — «в ядре/диспетчере».
static mut CURRENT: usize = MAX_TASKS;
/// С какой задачи начинать поиск следующей (round-robin).
static mut LAST: usize = 0;

extern "C" {
    fn switch_context(prev_rsp: *mut u64, next_rsp: u64);
}

unsafe fn push_word(sp: &mut u64, v: u64) {
    *sp -= 8;
    core::ptr::write_volatile(*sp as *mut u64, v);
}

/// Подготовить задачу: стек, кадр для switch_context, состояние Ready.
/// Возвращает false, если свободных слотов нет.
pub fn spawn(entry: extern "C" fn()) -> bool {
    unsafe {
        for i in 0..MAX_TASKS {
            if TASKS[i].state == State::Free {
                // Адрес вершины стека считаем через сырой указатель, чтобы не
                // создавать ссылку на `static mut` (lint static_mut_refs).
                let base = core::ptr::addr_of_mut!(STACKS) as *mut u8;
                let top = (base.add((i + 1) * STACK_SIZE) as u64) & !15;
                let mut sp = top;
                // Порядок обратен pop'ам в switch_context:
                // [top-8]=pad, [top-16]=ret, затем rbp, rbx, r12..r15.
                // Паддинг даёт rsp % 16 == 8 на входе в entry (как после call).
                push_word(&mut sp, 0);
                push_word(&mut sp, entry as usize as u64);
                push_word(&mut sp, 0); // rbp
                push_word(&mut sp, 0); // rbx
                push_word(&mut sp, 0); // r12
                push_word(&mut sp, 0); // r13
                push_word(&mut sp, 0); // r14
                push_word(&mut sp, 0); // r15
                TASKS[i].rsp = sp;
                TASKS[i].state = State::Ready;
                return true;
            }
        }
        false
    }
}

/// Крутить задачи, пока есть готовые. Возвращает число выполненных задач.
pub fn run() -> usize {
    let mut done = 0usize;
    unsafe {
        loop {
            let mut next = None;
            for k in 1..=MAX_TASKS {
                let i = (LAST + k) % MAX_TASKS;
                if TASKS[i].state == State::Ready {
                    next = Some(i);
                    break;
                }
            }
            let i = match next {
                Some(i) => i,
                None => break,
            };
            LAST = i;
            CURRENT = i;
            switch_context(core::ptr::addr_of_mut!(MAIN_RSP), TASKS[i].rsp);
            // Сюда возвращаемся после yield_now() или task_exit().
            if TASKS[i].state == State::Dead {
                done += 1;
            }
        }
        CURRENT = MAX_TASKS;
        done
    }
}

/// Отдать процессор. Вызывается только из задачи.
pub fn yield_now() {
    unsafe {
        let cur = CURRENT;
        if cur >= MAX_TASKS {
            return;
        }
        TASKS[cur].state = State::Ready;
        switch_context(core::ptr::addr_of_mut!(TASKS[cur].rsp), MAIN_RSP);
    }
}

/// Завершить текущую задачу. Управление возвращается в run() и уже оттуда
/// задача больше никогда не выбирается.
pub fn task_exit() -> ! {
    unsafe {
        let cur = CURRENT;
        if cur < MAX_TASKS {
            TASKS[cur].state = State::Dead;
            switch_context(core::ptr::addr_of_mut!(TASKS[cur].rsp), MAIN_RSP);
        }
    }
    loop {
        unsafe { core::arch::asm!("cli", "hlt") };
    }
}

/// (готовых, завершённых) — для диагностики.
#[allow(dead_code)]
pub fn stats() -> (usize, usize) {
    let mut ready = 0;
    let mut dead = 0;
    unsafe {
        let p = core::ptr::addr_of!(TASKS) as *const Task;
        for i in 0..MAX_TASKS {
            match (*p.add(i)).state {
                State::Ready => ready += 1,
                State::Dead => dead += 1,
                State::Free => {}
            }
        }
    }
    (ready, dead)
}

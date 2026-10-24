# STATE — текущее состояние проекта «ИрНа»

> Обновлять в конце каждой сессии. Это краткая выжимка; подробности — в `HISTORY.md`.

**Дата:** 2026-10-24
**Этап:** M1 (в работе)
**Ветка:** `main`
**Последний коммит:** `9922d60` — планировщик: проверка 3 задач в CI

## Что сделано

### M0 — тулчейн и загрузка
- Multiboot2-заголовок, переход real → protected → long mode.
- GDT, identity-map, загрузка через GRUB (BIOS).
- Rust `no_std`-ядро, вывод в COM1.
- Скрипты `ci/build.sh`, `ci/run-qemu.sh`.
- Проверено в QEMU: `M0 hello-world (serial console) OK`.

### M1 — базовое ядро (конвейер работает, MBI-дефект закрыт)
- `kernel/src/bootinfo.rs` — парсер MBI (memory map tag=6).
- `kernel/src/pmm.rs` — bitmap-аллокатор кадров.
- `kernel/src/gdt.rs` — GDT + TSS (RSP0, IST1).
- `kernel/src/idt.rs` — IDT: 48 gate-дескрипторов (0..31 исключения, 32..47 IRQ),
  общий обработчик, печать вектора/error/rip/cs/rflags в COM1, `#DF` на IST1.
  Smoke-тест: `int3` → `#BP` → обработчик → `iretq` (ядро продолжает работу).
- `kernel/src/io.rs` — `inb`/`outb`/`io_wait`.
- `kernel/src/pic.rs` — 8259A: remap на 0x20..0x2F, маски, EOI, чтение ISR.
- `kernel/src/pit.rs` — канал 0, режим 3, 100 Гц (делитель 11931), счётчик тиков,
  `wait_ticks` (спит в `hlt`).
- `kernel/src/irq.rs` — диспетчер IRQ, ложные IRQ7/IRQ15, счётчик неожиданных.
- `kernel/src/keyboard.rs` — PS/2 через IRQ1: 8042 (конфиг-байт, `0xAE`,
  `0xF4`), SPSC-кольцо 64 байта, декодер scancode set 1 (Shift, `0xE0`-
  расширения, стрелки/Home/End/Delete/PgUp/PgDn), `poll()` без блокировок.
  Раскладка пока US.
- `kernel/src/heap.rs` — bump-аллокатор на 1 МиБ, `#[global_allocator]`.
- `kernel/src/vga.rs` — текстовая консоль 80×25, UTF-8 → CP437 (fallback,
  если LFB недоступен). Кириллица в ROM-шрифте VGA отсутствует.
- `kernel/src/fb.rs` — консоль в линейном framebuffer: глифы 10×16, масштаб
  2× при высоте ≥ 700 px, перенос/прокрутка, 24 и 32 bpp.
- `kernel/src/font.rs` — 165 глифов 10×16 (PT Mono, OFL): ASCII, кириллица
  А-я + Ё/ё, `– — « »`. Генерируется `tools/gen-font8x16.ps1`.
- `kernel/src/console.rs` — диспетчер вывода: LFB, иначе VGA text.
- `kernel/src/paging.rs` — `map_identity_2m` для LFB (пул из 8 page tables)
  и `map_user_4k` (расщепление huge-страницы, U/S на всех уровнях).
- `kernel/src/syscall.rs` — `EFER.SCE`, `STAR/LSTAR/FMASK`, диспетчер
  `write`/`exit` (Linux-подобный ABI), `read` → `-ENOSYS`.
- `kernel/src/user.rs` — smoke-тест ring 3: страницы кода/стека с U/S,
  `write` кириллицей и `exit`, возврат в ядро.
- `kernel/src/sched.rs` — кооперативный планировщик: 8 задач, round-robin,
  `switch_context` (callee-saved + RSP), `yield_now`/`task_exit`.
- `kernel/src/arch.asm` — trampoline, identity map 0..1 ГиБ (huge pages,
  MMIO read-only), `gdt_reload`, стеки TSS.RSP0/IST1, information-request тег
  (type=1), request-тег framebuffer (type=5, 1024×768×32), копия MBI
  в `.mbi_copy` до включения paging. `header_length = 64`.
- `kernel/src/main.rs` — конвейер инициализации M1 + smoke-тесты.
- `kernel/linker.ld` — метка `_end` для PMM, секция `.mbi_copy`.

Фактический serial-вывод QEMU (после исправления MBI):

```text
ИрНа kernel: M1 boot start
MBI magic saved  = 920085129      ← 0x36D76289 (EAX)
MBI ptr copy     = 1048624        ← 0x100030 (безопасный буфер)
MBI ptr ebx orig = 2234720        ← адрес MBI от GRUB
MBI total size   = 1064
MBI found        = 1
MBI tags mask    = 2115447
MBI free_mem_top = 536739840      ← 0x1FFE0000 (QEMU -m 512)
MBI total_free   = 536345600
PMM frames total = 32768          ← реальная карта (cap 128 МиБ)
PMM frames free  = 32225
PMM alloc_frame OK
GDT+TSS loaded (RSP0, IST1)
IDT loaded (48 gates: 32 exception + 16 IRQ)

EXCEPTION 0x3 #BP breakpoint error=0x0 rip=0x10b6af cs=0x8 rflags=0x2
IDT smoke: #BP handled, returning
IDT int3 smoke OK
PIC remapped (IRQ 0x20..0x2F)
PIT programmed (100 Hz)
interrupts enabled (IF=1)
PIT: 20 ticks OK
PS/2 keyboard ready (IRQ1)
LFB addr         = 4244635648      ← 0xFD000000 (QEMU)
LFB pitch        = 2400
LFB width        = 800
LFB height       = 600
LFB bpp          = 24
framebuffer console OK (800x600, 80x37 chars)
kernel heap alloc OK (100 bytes)
framebuffer console OK
SYSCALL/SYSRET enabled (EFER.SCE, STAR/LSTAR/FMASK)
task B: tick 1
task C: tick 1
task A: tick 1
...
task B: done
task C: done
task A: done
sched: tasks done = 3
ИрНа kernel: M1 subsystems OK (bootinfo/PMM/GDT/IDT/PIC/PIT/heap/LFB/syscall/sched)
==================================================
ring 3: iretq в user mode, код по адресу 0x400000, стек 0x403000
Привет из ring 3! syscall/sysret OK
ring 3: syscall/sysret round-trip OK
keyboard echo: нажмите клавиши (serial + framebuffer)
key: 'a'
```

`./ci/run-qemu.sh check` → SUCCESS, `grub-file --is-x86-multiboot2` → rc=0.

### Исправление MBI (2026-10-24)
Корень дефекта оказался не в GRUB:
1. `bootinfo.rs` читал адрес переменной `mb_mboot_ptr`, а не её значение.
2. Magic Multiboot2 приходит в EAX; в самой структуре MBI по смещению 0 лежит
   `total_size` (наблюдаемое значение 1000 — это размер MBI, а не magic).
3. GRUB размещает MBI в области, пересекающейся с `.bss` ядра (в старой
   раскладке — внутри стека). Теперь MBI копируется в `.mbi_copy` (8 КиБ)
   в 32-битном trampoline до включения paging и до использования стека.
4. Тег information request исправлен: type=1 (не 0), с padding до 8 байт,
   `header_length=40`.

### IDT и исключения (2026-10-24)
- `arch.asm`: NASM-макросы `ISR_NOERR`/`ISR_ERR` → `isr_stub_0..31`; общий
  `isr_common` сохраняет 15 регистров, кладёт указатель на кадр в RDI,
  выравнивает стек (`and rsp, -16`) и вызывает `exception_handler`.
- `idt.rs`: 256 записей (заполнены 0..31), gate = 0x8E, селектор 0x08;
  `#DF` использует IST1. Обработчик печатает вектор, error code, RIP, CS, RFLAGS
  (для `#PF` — ещё CR2). `#BP` возвращает управление, остальные — `cli; hlt`.
- Проверено в QEMU: `EXCEPTION 0x3 ... rip=0x10b62f`, `IDT int3 smoke OK`,
  далее конвейер доходит до `M1 subsystems OK`.
- `ci/build.sh`: GRUB `timeout=0` — грузимся сразу, без меню. Меню с одним
  пунктом иногда зависало в QEMU/TCG (особенно при `terminal_input serial`)
  и ISO не загружался без нажатия Enter.

### PIC + PIT (2026-10-24)
- `pic.rs`: 8259A перепрошит с BIOS-диапазона 0x08..0x0F на 0x20..0x2F
  (master 0x20, slave 0x28, каскад на IRQ2), после init все линии замаскированы.
  Есть `mask`/`unmask`/`eoi`/`eoi_master_only`/`read_isr`.
- `pit.rs`: канал 0, режим 3, делитель 11931 → 100.007 Гц; атомарный счётчик
  тиков, `wait_ticks` спит в `hlt`.
- `irq.rs`: диспетчер для векторов ≥ 0x20; IRQ0 → `pit::tick()`; ложные
  IRQ7/IRQ15 определяются чтением ISR (master EOI только для IRQ15);
  неожиданные линии считаются.
- IDT расширена до 48 гейтов; общий stub `arch.asm` теперь зовёт
  `interrupt_handler` (исключения и IRQ в одной точке входа).
- Smoke-тест: `sti` + ожидание 20 тиков. Он проверяет сразу три вещи —
  приход IRQ0, корректный EOI (иначе второй тик не придёт) и возврат `iretq`.
- Serial: `PIC remapped`, `PIT programmed (100 Hz)`, `PIT: 20 ticks OK`.

### PS/2 клавиатура (2026-10-24)
- `keyboard.rs`: 8042 настраивается явно — конфиг-байт (`0x20`/`0x60`):
  bit0 IRQ1, bit6 трансляция set 2 → set 1, bit4 снят; затем `0xAE` (разрешить
  интерфейс) и `0xF4` (включить сканирование). Без `0xF4` клавиатура после POST
  остаётся в «scan disabled» и скан-коды не приходят вообще.
- IRQ1 только забирает байт из порта `0x60` в SPSC-кольцо (64 байта,
  `Acquire`/`Release`); разбор — в `poll()` главного потока. Байты мыши
  (status bit5) отбрасываются.
- Декодер set 1: make/break, префикс `0xE0`, Shift (`0x2A`/`0x36`), ASCII-таблица
  с учётом Shift; стрелки/Home/End/Delete/PgUp/PgDn — отдельные `Key`.
- `main.rs`: после `unmask(1)` — цикл `hlt` + `poll()` с эхом в serial и VGA.
- Проверено в QEMU (run `40080649501`): `PS/2 keyboard ready (IRQ1)`,
  затем `key: 'a'` после инъекции клавиши.
- Локальный стенд `ci/run-qemu.ps1`: `-display none`, скрытое окно, serial через
  TCP, клавиша — через QMP `input-send-event` (HMP `sendkey` headless не работает).

### Framebuffer (LFB) и кириллица (2026-10-24)
- `arch.asm`: request-тег framebuffer type=5 (`1024×768×32`), `header_length=64`;
  экспорт `pml4`/`pdpt`.
- `bootinfo.rs`: разбор тега 8 — только RGB (`fb_type=1`) и 24/32 bpp; позиции
  и размеры каналов сохраняются, порядок не предполагается.
- `paging.rs`: `map_identity_2m(phys, len)` — 2-МиБ страницы RW, пул из 8
  page tables в `.bss`, перезагрузка CR3. Нужен потому, что LFB у QEMU лежит
  на `0xFD000000` — выше 1 ГиБ, покрытого trampoline-картой.
- `font.rs`: 165 глифов 10×16 из PT Mono (OFL): ASCII, кириллица А-я, Ё/ё,
  `– — « »`; генератор `tools/gen-font8x16.ps1`, превью —
  `tools/preview-font8x16.ps1`.
- `fb.rs`: консоль в LFB — фон/текст, масштаб 2× при высоте ≥ 700, `\t`,
  backspace, прокрутка сдвигом; 24 и 32 bpp.
- `console.rs`: LFB, если доступен, иначе VGA text.
- Проверено (run `40083627944`): `framebuffer console OK (800x600, 80x37 chars)`,
  скриншот QMP подтверждает кириллицу и типографику.
- GRUB сам выбирает режим по полям тега (в этом окружении — 800×600×24);
  ядро к конкретному режиму не привязано.

### SYSCALL/SYSRET и ring 3 (2026-10-24)
- GDT: user-data `0x18|3` перед user-code `0x20|3` — требование SYSRET
  (`CS = STAR[63:48]+16`, `SS = +8`).
- MSR: `EFER.SCE`, `STAR = 0x13<<48 | 0x08<<32`, `LSTAR = syscall_entry`,
  `FMASK = 0x47700`.
- `syscall_entry` (arch.asm): сохраняет user RSP, переключается на стек 16 КиБ,
  сохраняет RCX/R11 и 14 регистров, зовёт `syscall_dispatch`, возвращает
  `sysretq`. Вложенные syscall из ring 0 не поддерживаются (M1).
- `enter_user_mode` — `iretq` в ring 3; `syscall_exit_to_kernel` —
  возврат на основной стек + `sti` + `kernel_after_user`.
- `map_user_4k` расщепляет 2-МиБ huge-страницу и ставит U/S на всех уровнях
  (без этого — `#PF error=0x5` при выборке инструкции из ring 3).
- Проверено (run `40084859952`): сообщение из ring 3 на кириллице в serial
  и LFB, `syscall/sysret round-trip OK`, затем IRQ1 продолжает работать.

### Кооперативный планировщик (2026-10-24)
- `sched.rs`: 8 слотов, стеки 16 КиБ в `.bss`, состояния Free/Ready/Dead,
  round-robin. `spawn` собирает кадр (паддинг + адрес возврата + rbp/rbx/r12..r15),
  чтобы первый `ret` в `switch_context` попал в задачу с правильным
  выравниванием стека.
- `switch_context` (arch.asm): callee-saved SysV + обмен RSP.
- `yield_now` возвращает управление диспетчеру, `task_exit` помечает задачу
  Dead; `run()` возвращает число завершённых.
- Проверено (run `40085331878`): 3 задачи × 3 тика, `sched: tasks done = 3`,
  затем ring 3 и IRQ1 работают.
- Ограничения: без вытеснения, приоритетов и смены CR3 — это M2.

### Восстановление репозитория (2026-10-24)
- Сравнены `workspace_last` (M1) и `irna-os-main` (M0).
- Подтверждено: M0-снимок полностью входит в M1-рабочую область.
- Создан чистый репозиторий `C:\Projects\OS\irna-os` (31 файл).
- Подключён GitHub, выполнен push, автор — `Kay`.
- Удалён ошибочный `workspace2.zip`, `LICENSE` обновлён на `Copyright (c) 2026 Kay`.

### CI и тест (2026-10-24)
- GitHub Actions workflow собран успешно (run `40068792982`).
- Артефакт `irna-iso` скачан, `irna.iso` ≈ 11.9 МБ.
- Проверка в QEMU на ноутбуке: `M1 subsystems OK`, `MBI found = 1`,
  реальная карта памяти (`free_mem_top = 0x0FFE0000`).
- Рабочий цикл: push → Actions → artifact → QEMU.

## Следующие шаги (M1)

1. Вытесняющий планировщик по PIT (переключение из IRQ0).
2. Раскладка EN/RU (шрифт с кириллицей уже в ядре).
3. `copy_from_user` и проверки указателей для syscall.

## Известные проблемы и ограничения

- В этом окружении GRUB отдаёт 800×600×24 вместо запрошенных 1024×768×32
  (VBE-режим недоступен). Ядро читает bpp/pitch/каналы из тега 8 и работает
  с любым из них; на реальном железе режим может отличаться.
- PMM сохраняет детерминированный fallback 128 МиБ на случай, если MBI не
  разобран; в QEMU теперь используется реальная карта памяти.
- VGA text остаётся только fallback: кириллицы в ROM-шрифте (CP437) нет.
  Основной путь — LFB, там кириллица работает.
- Пул page tables в `paging.rs` — 8 таблиц (32 КиБ), без освобождения.
  Для M1 достаточно; полноценный VM-менеджер — в M2.
- SYSCALL: статический стек и статическая переменная user RSP → без вложенных
  syscall; `write` читает буфер пользователя напрямую (SMAP выключен,
  `copy_from_user` — M2).
- Тестовая user-программа — машинный код в `user.rs`; ELF-загрузчик и
  адресные пространства — M2. NXE не включён, страницы исполняемы.
- Планировщик кооперативный: без вытеснения, приоритетов и смены CR3;
  все задачи — функции ядра, стеки статические (8 × 16 КиБ).
- Полная сборка ISO требует Linux-тулчейн (GRUB, xorriso) → делается в CI.
- На ноутбуке нет Rust/NASM; локально только QEMU для тестов.
- Ноутбук: 4 ГБ RAM, Celeron N2840 → WSL2/VM не использовать.
- Репозиторий на GitHub публичный.

## Окружение

| Компонент | Где |
|---|---|
| Git | `C:\Projects\OS\tools\MinGit\cmd\git.exe` |
| GitHub CLI | `C:\Projects\OS\tools\gh\bin\gh.exe` |
| QEMU | `C:\Projects\OS\tools\qemu\qemu-system-x86_64.exe` |
| CI | GitHub Actions, Ubuntu, артефакт `irna-iso` |
| Тест | QEMU на Windows (TCG, без виртуализации) |

## Правило обновления

В конце сессии:
1. Дописать раздел в `HISTORY.md`.
2. Обновить этот файл (что сделано / что дальше / проблемы).
3. При необходимости — `ROADMAP.md` и `DECISIONS.md`.
4. Сделать коммит и push.

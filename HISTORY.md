# HISTORY — журнал работ по проекту «ИрНа»

> Файл обновляется после каждого этапа (M0, M1, ...) или по запросу заказчика.
> Формат: дата → этап → что сделано → проблемы и их корневые причины → итог.

---

## 2026-10-18/19 — Подготовка проекта (до M0)

- Согласованы цель, стек и лицензия (см. CONTEXT.md, SPEC.md v0.2, DECISIONS.md D-001…D-012).
- Создан приватный репозиторий GitHub: `glybina18diver-lang/irna-os` (2 коммита).
- Названия ОС: «ИрНа» (вымышленное), бренд вынесен в `branding.md`.
- ROADMAP: M0 (тулчейн+hello-kernel) → M1 (фреймбуфер, текст, PS/2) → M2 (память, планировщик, ELF) → M3 (window server) → M4 (рабочий стол) → M5 (приложения).

---

## 2026-10-23 — Этап M0: тулчейн и загрузка hello-ядра ✅ ЗАВЕРШЁН

### Артефакты
- `kernel/src/arch.asm` — Multiboot2-заголовок + переход real→protected→long mode + GDT.
- `kernel/src/main.rs`, `serial.rs`, `brand.rs` — Rust-ядро: COM1-вывод баннера M0.
- `kernel/build.rs` — сборка arch.asm через nasm (-f elf64), линковка с linker.ld.
- `.cargo/config.toml` — target x86_64-unknown-none, `-C relocation-model=static`, `-no-pie`.
- `ci/build.sh` — cargo build -Zbuild-std → проверка grub-file → grub-mkrescue ISO.
- `ci/run-qemu.sh` — запуск ISO в QEMU, serial пишется в файл, grep-проверка баннера.
- Зависимости контейнера: qemu-system-x86, nasm, xorriso, grub-common/grub-mkrescue, rustup (nightly).

### Хронология отладки загрузки ISO (4 итерации)

1. **v1 — битый MB2-заголовок.** Ошибки в длине заголовка и checksum → GRUB не видел ядро.
   Исправлено: header_length пересчитан, checksum = 0x10000000 − сумма.
2. **v2 — сборка.** ci/build.sh переписан на `cargo build -Zbuild-std` (no_std, без std-зависимостей).
3. **v3 — зависание после long mode.** Корень: в `gdt64_ptr` поле base = 0 вместо адреса `gdt64`
   → #GP / triple fault при far jump. Исправлено: `dd gdt64`.
4. **v4 — `GRUB error: unsupported tag: 0x14`.** Теги MB2 были в смешанном формате dw/dd;
   переписал все теги в канонический `dd type / dd size`. Но Debian-сборка GRUB 2.06 всё равно
   отвергала framebuffer/console-теги (неполная поддержка request-тегов, даже с optional-битом).
   **Корневое решение для M0:** минимальный заголовок — magic/arch/length/checksum + terminator
   (header_length = 24, checksum 0x17ADAF12). Serial-порт ядро инициализирует само (COM1),
   точка входа — из ELF entry. Тег module alignment (type=6) удалён.

### Итоговые проверки (финальный прогон 2026-10-23)
```
grub-file --is-x86-multiboot2 build/irna-kernel.elf   → rc=0 (OK)
hex .multiboot_header: d65052e8 00000000 18000000 12afad17 00000000 08000000
./ci/run-qemu.sh check → SUCCESS, serial-лог:
    Booting `IrNa OS (M0)'
    ИрНа kernel: M0 hello-world (serial console) OK
    brand name utf8 bytes: d098d180d09dd0b0  («ИрНа» UTF-8 читается)
build/irna.iso — 4.9M, bootable via grub-mkrescue (BIOS/i386 PC)
```
Примечание: `grub-file --is-x86-multiboot2` проверяет ELF-образы, а не ISO; проверка выполняется
на `build/irna-kernel.elf` (он же лежит в ISO как /boot/irna-kernel.elf).

### Коммиты Git
- `ab6dd23` — fix v4: minimal MB2 header, GDT base, serial grub.cfg
- `3a6fda8` — M0 final verification — build + QEMU check SUCCESS

### Уроки / ограничения окружения
- GRUB 2.06 (Debian 12): request-теги MB2 (framebuffer type=5, console flags type=3) ведут к
  «unsupported tag» → на M1 для фреймбуфера понадобится либо свой загрузчик/ограничения GRUB,
  либо linear framebuffer через VESA-режим самого GRUB (`set gfxpayload=keep`).
- Serial-лог QEMU содержит ANSI-последовательности → проверять через `strings`/`grep -a`.
- В run-qemu.sh serial пишется в файл (`-serial file:`), а не через pipe+tee (tee терял вывод
  при таймауте).

### Статус: M0 закрыт. Готовность к M1: фреймбуфер, VGA-текст, клавиатура/мышь PS/2.

---

## 2026-10-23/24 — Этап M1: базовое ядро (MBI-дефект закрыт)

### Что сделано и работает
ISO пересобран, ядро загружается в QEMU через GRUB Multiboot2 и проходит
конвейер инициализации M1. Актуальный serial-вывод после исправления MBI:

```text
ИрНа kernel: M1 boot start
MBI magic saved  = 920085129      ← 0x36D76289 (EAX)
MBI ptr copy     = 1048624        ← 0x100030 (безопасный буфер)
MBI ptr ebx orig = 2225208        ← 0x21F438 (GRUB)
MBI total size   = 1064
MBI found        = 1
MBI tags mask    = 2115447
MBI free_mem_top = 268304384      ← 0x0FFE0000
MBI total_free   = 267910144
PMM frames total = 32768          ← реальная карта (cap 128 МиБ)
PMM frames free  = 32227
PMM alloc_frame OK
GDT+TSS loaded (RSP0, IST1)
kernel heap alloc OK (100 bytes)
VGA text console OK             ← кириллица пока не работает (CP437)
ИрНа kernel: M1 subsystems OK (bootinfo/PMM/GDT/heap/VGA)
```

Проверки: `./ci/run-qemu.sh check` → SUCCESS (rc=0);
`grub-file --is-x86-multiboot2 build/irna-kernel.elf` → rc=0.

### Созданные/изменённые файлы
- `kernel/src/arch.asm` — trampoline: сохранение EAX/EBX, копия MBI в
  `.mbi_copy` до включения paging, карта памяти 0–1 ГиБ (huge pages,
  MMIO read-only), экспорт `gdt_reload`, стеки для TSS.RSP0/IST1,
  information-request тег (type=1, padding, `header_length=40`).
- `kernel/src/bootinfo.rs` — парсер MBI (memory map tag=6).
- `kernel/src/pmm.rs` — bitmap-аллокатор кадров.
- `kernel/src/gdt.rs` — GDT+TSS (user-селекторы заложены под M2 syscall).
- `kernel/src/heap.rs` — kmalloc-куча как `#[global_allocator]`.
- `kernel/src/vga.rs` — текстовая консоль 80×25, UTF-8 → CP437.
- `kernel/src/main.rs` — конвейер инициализации M1 + smoke-тесты.
- `kernel/linker.ld` — метка `_end` для PMM.
- `ci/build.sh` — `-Zbuild-std=core,alloc,panic_abort`, menuentry «IrNa OS (M1)».
- `ci/run-qemu.sh` — проверка строки «M1 subsystems OK».

### Закрытый дефект: MBI memory map (2026-10-24)
Корень оказался не в GRUB, а в трёх местах:
1. `bootinfo.rs` брал `&raw const mb_mboot_ptr` — адрес самой переменной,
   а не её значение (адрес MBI).
2. Magic Multiboot2 передаётся в EAX и в структуре MBI отсутствует: по
   смещению 0 лежит `total_size` (наблюдаемые 1000 — это размер MBI).
3. GRUB размещает MBI в области, пересекающейся с `.bss` ядра (в старой
   раскладке — внутри стека). При росте стека MBI мог быть затёрт.

Исправление:
- `arch.asm` сохраняет EAX (`mb_mboot_magic`) и EBX (`mb_mboot_ebx`).
- MBI копируется в `.mbi_copy` (8 КиБ) в начале образа **до** включения
  paging и до использования стека; `mb_mboot_ptr` указывает на копию.
- `bootinfo.rs` читает значение `mb_mboot_ptr`, проверяет magic из EAX,
  берёт `total_size` из offset 0 и разбирает теги с offset 8.
- information-request тег исправлен: type=1 (type=0 — end tag), добавлен
  padding до 8 байт, `header_length=40`.

Результат: `MBI found = 1`, `free_mem_top = 0x0FFE0000`,
`total_free = 267910144`; PMM использует реальную карту.
Fallback 128 МиБ оставлен как защитная ветка.

### Открытый дефект: кириллица в VGA
В VGA text mode кириллица отображается «крякозябрами». Причина: ROM-шрифт VGA
(CP437) не содержит кириллических глифов, а таблица в `vga.rs` отображает
буквы в коды псевдографики/латиницы. Serial-вывод при этом корректный (UTF-8).
Решение: свой 8×16 шрифт с кириллицей, загружаемый в VGA plane 2, либо
временная ASCII-заглушка в VGA.

### Дальше
Кириллица в VGA отложена до framebuffer (решение заказчика). Далее:
IDT + PIC remap, PIT-таймер, PS/2 клавиатура, framebuffer/syscall.

---

## Настройки репозитория (24.10.2026)
- Автор коммитов и владелец репозитория: **glybina18diver-lang** `<glybina18diver-lang@users.noreply.github.com>`.
- Все исторические коммиты переписаны (`git filter-branch`), глобальный `git config` обновлён — все будущие коммиты автоматически пишутся от этого автора.

## 2026-10-24 — восстановление репозитория и объединение баз

### Контекст
После удаления и повторного создания репозитория на GitHub локальная связь с ним была потеряна.
Остались два снимка: `workspace_last` (рабочая область агента, M1) и `irna-os-main`
(скачанный репозиторий до очистки, M0).

### Сравнение
- `irna-os-main`: 25 файлов, этап M0.
- `workspace_last`: 31 исходный файл (плюс 116 артефактов `build/`), этап M1.
- Файлов, которые есть только в M0-снимке, нет: он полностью входит в M1.
- Отличаются 6 файлов (`main.rs`, `arch.asm`, `serial.rs`, `linker.ld`, `ci/build.sh`,
  `ci/run-qemu.sh`) — в M1 они новее.
- Только в M1: `HISTORY.md`, `bootinfo.rs`, `gdt.rs`, `heap.rs`, `pmm.rs`, `vga.rs`.

### Что сделано
- Создан чистый репозиторий `C:\Projects\OS\irna-os` (31 файл, без `build/` и старого `.git`).
- Инициализирован git, ветка `main`, автор — `Kay <glybina18diver-lang@users.noreply.github.com>`.
- Подключён remote `https://github.com/glybina18diver-lang/irna-os.git`, выполнен push.
- Удалён ошибочный `workspace2.zip`; `LICENSE` приведён к `Copyright (c) 2026 Kay`.
- Добавлены `AGENTS.md`, `docs/STATE.md`, `docs/BUILD.md`, `docs/HANDOFF.md`.
- Добавлен GitHub Actions workflow `.github/workflows/build.yml` (сборка ISO + артефакты).
- Установлен портативный QEMU; Git и GitHub CLI уже портативные.
- WSL2 и виртуальные машины на ноутбуке не используются.

### Коммиты
- `2d9b88e` — M0+M1: восстановленная полная кодовая база.
- `2155f34` — CI + документация для агентов.
- `21ace7e` — состояние M1 и детали дефекта MBI.
- `a865561` — CI pipeline проверен, артефакт ISO протестирован в QEMU.
- `061f57e` — зафиксирован дефект кириллицы в VGA text mode.
- `694c14d` — исправлен разбор Multiboot2 MBI (magic из EAX, копия до paging).
- `fc5fa6a` — выравнивание information-request тега (header_length=40).

### CI и тест
- GitHub Actions workflow отработал успешно (run `40068792982`).
- Артефакт `irna-iso` скачан: `C:\Projects\OS\artifacts\fix2\irna.iso` ≈ 11.9 МБ.
- ISO проверен в QEMU на ноутбуке: `M1 subsystems OK`, `MBI found = 1`,
  реальная карта памяти (`free_mem_top = 0x0FFE0000`).
- Рабочий цикл: push → Actions → artifact → QEMU.

### Дальше
M1: PIC/PIT, PS/2, framebuffer, syscall, планировщик.

---

## 2026-10-24 — M1: IDT и обработчики исключений

### Что сделано
- `kernel/src/idt.rs` (новый): IDT на 256 записей, заполнены векторы 0..31,
  gate-дескрипторы 0x8E (present, DPL=0, 64-bit interrupt gate), селектор 0x08.
  `#DF` (8) направлен на IST1, остальные — на текущий стек ядра.
- `kernel/src/arch.asm`: NASM-макросы `ISR_NOERR`/`ISR_ERR` генерируют
  `isr_stub_0..31` (для векторов с error code — без искусственного нуля),
  общий `isr_common` сохраняет 15 регистров, передаёт `InterruptFrame*` в RDI,
  выравнивает стек по SysV и после обработчика делает `iretq`. Таблица адресов —
  `isr_stub_table` (32 × u64) в `.rodata`.
- `kernel/src/serial.rs`: `write_hex_u64`.
- `kernel/src/main.rs`: `idt::init()` + smoke-тест `int3`.
- `ci/build.sh`: GRUB `timeout=0` вместо `1`.

### Результат в QEMU (run 40071614155, ISO из CI)
```text
GDT+TSS loaded (RSP0, IST1)
IDT loaded (32 exception gates)

EXCEPTION 0x3 #BP breakpoint error=0x0 rip=0x10b62f cs=0x8 rflags=0x2
IDT smoke: #BP handled, returning
IDT int3 smoke OK
...
ИрНа kernel: M1 subsystems OK (bootinfo/PMM/GDT/IDT/heap/VGA)
```
`#BP` — ловушка (trap): CPU сохраняет RIP *после* `int3`, поэтому обработчик
возвращается без правки RIP. `rflags=0x2` — IF=0, прерывания ещё запрещены.

### Дефект стенда (не ядра): зависание меню GRUB
Локальные прогоны иногда останавливались на отрисовке меню GRUB: отсчёт
«will be executed automatically in 1s» не запускался, ISO не грузился без
нажатия Enter (в т.ч. при `-serial file:` и `-serial tcp:`). Это стенд, а не
ядро: с отправкой CR по serial загрузка проходила и ядро работало штатно.
Решение — `set timeout=0` в `ci/build.sh`: единственный пункт меню не нужен,
загрузка сразу. Дополнительно сделан локальный скрипт
`tools/run-qemu-test.ps1` (serial через TCP + CR в меню, очистка ANSI).

### Коммиты
- `9d0db8a` — `kernel: IDT + обработчики исключений (векторы 0..31), smoke-тест int3`.

### Дальше
PIC (8259) remap на 0x20..0x2F + PIT (IRQ0, 100 Гц), затем PS/2.

---

## 2026-10-24 — M1: PIC remap и PIT 100 Гц

### Что сделано
- `kernel/src/io.rs` — `inb`/`outb`/`io_wait` (порты x86-64).
- `kernel/src/pic.rs` — 8259A: ICW1..ICW4, master → 0x20, slave → 0x28,
  каскад на IRQ2, маска 0xFF после init; `mask`/`unmask`/`eoi`/
  `eoi_master_only`/`read_isr`.
- `kernel/src/pit.rs` — канал 0, режим 3 (square wave), делитель
  `1 193 182 / 100 = 11931` → 100.007 Гц; `AtomicU64` счётчик тиков;
  `wait_ticks(n)` спит в `hlt`.
- `kernel/src/irq.rs` — диспетчер IRQ: IRQ0 → `pit::tick()`, EOI после
  обработки; ложные IRQ7 (без EOI) и IRQ15 (EOI только master) детектируются
  чтением регистра ISR; неожиданные линии считаются.
- `kernel/src/idt.rs` — IDT расширена до 48 гейтов (0..31 исключения,
  32..47 IRQ); общий stub вызывает `interrupt_handler`, который для векторов
  ≥ 0x20 уходит в `irq::dispatch`.
- `kernel/src/arch.asm` — 16 новых заглушек `isr_stub_32..47` и записи в
  `isr_stub_table`.

### Результат в QEMU (run 40075980853)
```text
IDT loaded (48 gates: 32 exception + 16 IRQ)
EXCEPTION 0x3 #BP breakpoint error=0x0 rip=0x10b6af cs=0x8 rflags=0x2
IDT smoke: #BP handled, returning
IDT int3 smoke OK
PIC remapped (IRQ 0x20..0x2F)
PIT programmed (100 Hz)
interrupts enabled (IF=1)
PIT: 20 ticks OK
...
ИрНа kernel: M1 subsystems OK (bootinfo/PMM/GDT/IDT/PIC/PIT/heap/VGA)
```

### Почему smoke-тест ждёт именно 20 тиков
Ожидание 20 тиков проверяет не только «прерывание пришло», но и EOI:
без подтверждения PIC не поднимет IRQ0 повторно, и счётчик остановился бы
на 1. Дополнительно это проверяет, что `iretq` восстанавливает контекст —
ядро продолжает работу после каждого прерывания.

### Коммиты
- `aeebf0c` — `kernel: PIC remap + PIT 100 Hz, IRQ0 через общий stub`.

### Дальше
PS/2 клавиатура (IRQ1), затем framebuffer.

---

## 2026-10-24 — M1: PS/2 клавиатура (IRQ1)

### Что сделано
- `kernel/src/keyboard.rs` (новый):
  - инициализация 8042: слить мусор из буфера, прочитать конфигурационный байт
    (`0x20`), выставить bit0 (IRQ1) и bit6 (трансляция set 2 → set 1), снять
    bit4 («клавиатура отключена»), записать обратно (`0x60`);
  - `0xAE` — разрешить интерфейс клавиатуры, `0xF4` — включить сканирование;
  - SPSC-кольцо 64 байта: IRQ1 только читает `0x60` и кладёт байт в буфер
    (`Acquire`/`Release`, без блокировок), разбор — в `poll()` главного потока;
  - декодер set 1: make/break (`| 0x80`), префикс `0xE0` для стрелок/Home/End/
    Delete/PgUp/PgDn, состояние Shift (`0x2A`/`0x36`), таблица ASCII с Shift;
  - `handle_irq()` отбрасывает байты мыши (status bit5), чтобы они не застревали.
- `kernel/src/irq.rs`: IRQ1 → `keyboard::handle_irq()`, EOI после обработки.
- `kernel/src/main.rs`: `keyboard::init()` + `pic::unmask(1)` в конвейере,
  затем цикл `hlt` + `keyboard::poll()` с эхо-выводом в serial и VGA.
- `ci/run-qemu.sh`: проверки `PS/2 keyboard ready`, `key: '<символ>'`.
- `ci/run-qemu.ps1` (новый локальный стенд под Windows): serial через TCP,
  `-display none`, `-WindowStyle Hidden`, ввод клавиши через QMP.

### Результат в QEMU (run 40080649501, ISO из CI)
```text
PS/2 keyboard ready (IRQ1)
...
ИрНа kernel: M1 subsystems OK (bootinfo/PMM/GDT/IDT/PIC/PIT/heap/VGA)
==================================================
keyboard echo: нажмите клавиши (serial + VGA)
key: 'a'
```

### Корневые причины двух дефектов стенда
1. **Скан-коды не приходили вообще.** После POST клавиатура может остаться в
   состоянии «scan disabled» (`0xF5`), а конфигурационный байт 8042 сам по себе
   сканирование не включает. Решение: явная команда `0xF4` в `init()`; ответ
   `0xFA` приходит через IRQ1 и служит проверкой живости линии.
2. **HMP `sendkey` в headless-режиме не доходит до PS/2.** При `-display none`
   HMP-команда молча ничего не делает. Рабочий путь — QMP
   `input-send-event` с `events=[{type:"key",data:{down:true,key:{type:"qcode",
   data:"a"}}}, ...]`. Параметр `console` в QEMU 11 не принимается
   (`Parameter 'console' is unexpected`) — событие уходит в активную консоль.

### Замечания
- 8042 в QEMU/SeaBIOS отдаёт конфигурационный байт `0x61` (IRQ1 + трансляция
  включены, clock мыши выключен) — совпадает с ожидаемой раскладкой битов.
- Раскладка пока US; EN/RU и кириллица — после framebuffer (см. D-014).
- Локальный serial-лог в UTF-8 корректен; «крякозябры» в консоли PowerShell —
  только отображение (кодовая страница консоли), файл не повреждён.

### Коммиты
- `46a9b4e` — `kernel: PS/2 клавиатура (IRQ1, scancode set 1, Shift, стрелки)`.
- `79750ca` — `kernel: диагностика IRQ1 (cfg 8042, маска PIC, сырые скан-коды)`.
- `5bbbe71` — `kernel: PS/2 — включить сканирование (0xF4) в init`.
- `b35df6d` — `kernel: PS/2 — убрать диагностику IRQ1, включить write_config`.
- `250b956` — `ci: локальный прогон QEMU — ввод клавиши через QMP input-send-event`.

### Дальше
Framebuffer / LFB (снимает ограничение VGA text и разблокирует кириллицу),
затем `syscall`/`sysret`.

---

## 2026-10-24 — M1: framebuffer (LFB) и кириллица в консоли

### Что сделано
- `kernel/src/arch.asm`:
  - в Multiboot2-заголовок добавлен request-тег framebuffer (type=5) с
    `width=1024, height=768, depth=32`; `header_length` пересчитан с 40 до 64
    (16 + 12 + 4 + 20 + 4 + 8, теги выровнены по 8 байт);
  - экспортированы `pml4`/`pdpt` (нужны `paging.rs`).
- `kernel/src/bootinfo.rs` — разбор тега 8 (framebuffer): принимается только
  `fb_type=1` (RGB) и `bpp` 24/32; поля `red/green/blue_pos/size` сохраняются,
  порядок каналов не предполагается. Требование `sz >= 38` — фактический
  минимум полей тега.
- `kernel/src/paging.rs` (новый) — `map_identity_2m(phys, len)`: достраивает
  identity-map страницами по 2 МиБ с флагом RW. Page tables берутся из
  статического пула (8 таблиц = 32 КиБ в `.bss`), PDPT/PML4 — из `arch.asm`.
  После правки — перезагрузка CR3 (сброс TLB).
- `kernel/src/font.rs` (новый, генерируется) — 165 глифов 10×16: ASCII 0x20..0x7E,
  кириллица А-я, Ё/ё, типографика `– — « »`. Формат — `[[u16; 16]; 165]`,
  старший бит = левый пиксель.
- `kernel/src/fb.rs` (новый) — консоль в LFB: заливка фона, вывод глифов с
  масштабом (2× при высоте ≥ 700 px), перенос строки, `\t`, backspace,
  прокрутка сдвигом пикселей; поддержка 24 и 32 bpp.
- `kernel/src/console.rs` (новый) — единая точка вывода: LFB, если он есть,
  иначе VGA text. Serial остаётся независимым журналом.
- `kernel/src/main.rs` — инициализация LFB (диагностика addr/pitch/width/
  height/bpp в serial), динамическая подпись консоли в эхе клавиатуры.
- `tools/gen-font8x16.ps1` — растеризация PT Mono (OFL) в сетку 10×16
  (`SingleBitPerPixelGridFit`, без сглаживания); `tools/preview-font8x16.ps1` —
  превью готового `.rs` для проверки глазами. Оба скрипта — инструменты
  разработки, в ядро не входят.
- `assets/fonts/OFL-PT-Mono.txt` — текст лицензии SIL OFL 1.1.
- `ci/build.sh` — `insmod all_video` (без него GRUB не переключает режим);
  `ci/run-qemu.sh` — проверка `framebuffer console OK`;
  `ci/run-qemu.ps1` — `screendump` через QMP и клавиша по событию.

### Результат в QEMU (run 40083627944, ISO из CI)
```text
PS/2 keyboard ready (IRQ1)
LFB addr         = 4244635648     ← 0xFD000000
LFB pitch        = 2400
LFB width        = 800
LFB height       = 600
LFB bpp          = 24
framebuffer console OK (800x600, 80x37 chars)
kernel heap alloc OK (100 bytes)
ИрНа kernel: M1 subsystems OK (bootinfo/PMM/GDT/IDT/PIC/PIT/heap/LFB)
keyboard echo: нажмите клавиши (serial + framebuffer)
key: 'a'
```
Скриншот (`screendump`, PNG) подтверждает: кириллица, «—», «Ё/ё» и ««»»
рисуются корректно; прокрутка и перенос строк работают.

### Корневая причина «unsupported tag: 0x14» (снята)
Прежний вывод «GRUB 2.06 не поддерживает request-теги framebuffer» был
**неверным**. Причина — битый заголовок M0: теги были собраны в смешанном
формате `dw`/`dd`, из-за чего GRUB читал мусорные типы (0x14 = тип 20, которого
в заголовке не было). После приведения всех тегов к каноническому виду
(`dd type` / `dd size`, выравнивание по 8 байт) GRUB 2.06 корректно принимает
тег type=5 и сам переключает видеорежим: `grub-core/loader/multiboot.c`
формирует из полей тега `gfxpayload` и вызывает `set_console`. Поэтому
`set gfxpayload=...` в `grub.cfg` не нужен — значения берутся из тега.

### Почему GRUB отдал 800×600×24 вместо 1024×768×32
В этом окружении (SeaBIOS + `all_video`) запрошенный режим недоступен: VBE
предлагает 800×600×24. Ядро не полагается на конкретный режим — bpp, pitch и
позиции каналов читаются из тега 8, поэтому 24 bpp поддерживается наравне
с 32. Масштаб шрифта выбирается по высоте: 600 px → 1×, 80×37 символов.

### Дефекты, найденные при отладке
1. **`paging.rs`: локальная переменная затеняла статик `pdpt`** — компилятор
   отвергал `pdpt.add(...)`. Переименовано в `pdpt_tbl` (`4d2b0ff`).
2. **Шрифт 8×16 обрезал PT Mono по правому краю** — PT Mono шире 8 px при
   кегле 12. Ячейка расширена до 10×16, строки стали `u16` (`8f6779f`).
3. **Инъекция клавиши по таймеру промахивалась** — с видеорежимом загрузка
   под TCG занимает больше 8 с. Стенд теперь ждёт строку `keyboard echo:`
   в serial и только затем шлёт клавишу и скриншот (`4e7bb00`).
4. **PowerShell 5.1 читает `.ps1` как ANSI** — оба скрипта сохранены с UTF-8 BOM.

### Коммиты
- `602177c` — `kernel: LFB-консоль 1024x768x32, шрифт PT Mono 10x16 с кириллицей`.
- `aa98be9` — `ci: проверки framebuffer, screendump через QMP, all_video в grub.cfg`.
- `4d2b0ff` — `kernel: paging — не затенять статик pdpt локальной переменной`.
- `8f6779f` — `kernel: глифы типографики (—, «, ») и честная подпись консоли в эхе`.
- `4e7bb00` — `ci: клавиша и скриншот по событию 'keyboard echo:', а не по таймеру`.

### Дальше
`syscall`/`sysret` + libc-mini (`write`/`read`/`exit`), затем кооперативный,
а после — вытесняющий планировщик. Раскладка EN/RU теперь разблокирована:
шрифт с кириллицей уже в ядре.

---

## 2026-10-24 — M1: SYSCALL/SYSRET и первый выход в ring 3

### Что сделано
- `kernel/src/gdt.rs`: порядок user-сегментов приведён к требованию SYSRET —
  user-data `0x18|3` (индекс 3), user-code `0x20|3` (индекс 4). Процессор
  вычисляет `CS = STAR[63:48]+16`, `SS = STAR[63:48]+8`, поэтому data обязана
  идти перед code.
- `kernel/src/syscall.rs` (новый):
  - MSR: `EFER.SCE=1`, `STAR = 0x13<<48 | 0x08<<32`,
    `LSTAR = syscall_entry`, `FMASK = 0x47700` (TF|IF|DF|NT|AC);
  - диспетчер `syscall_dispatch` с Linux-подобным ABI (номер в RAX,
    аргументы RDI/RSI/RDX/R10/R8/R9, результат в RAX, ошибка — `-errno`);
  - `write(fd, buf, len)` для fd 1/2 (serial + консоль), `exit(code)`,
    `read` → `-ENOSYS` до появления блокирующего ввода.
- `kernel/src/user.rs` (новый) — smoke-тест ring 3: две 4-КиБ страницы
  (код и стек) с U/S=1, код `write(1, msg, len); exit(0)`, сообщение на
  кириллице. После `sys_exit` управление возвращается в ядро.
- `kernel/src/paging.rs` — `map_user_4k(virt, phys, writable)`:
  - расщепляет 2-МиБ huge-страницу на PT (identity-отображение сохраняется);
  - выставляет U/S **на всех уровнях** (PML4, PDPT, PD, PT).
- `kernel/src/arch.asm`:
  - `syscall_entry` — сохраняет user RSP, переключается на отдельный стек
    16 КиБ, сохраняет 16 регистров (включая RCX/R11), зовёт диспетчер,
    восстанавливает контекст и делает `sysretq`;
  - `syscall_exit_to_kernel` — возврат на основной стек ядра + `sti` +
    переход в `kernel_after_user`;
  - `enter_user_mode` — `iretq` в ring 3 (SS=0x1B, CS=0x23, IF=1),
    регистры обнуляются.
- `kernel/src/main.rs` — `syscall::init()` в конвейере, цикл эха вынесен
  в `pub fn echo_loop()`, куда возвращается `kernel_after_user`.
- `ci/run-qemu.{sh,ps1}` — проверки `SYSCALL/SYSRET enabled` и
  `syscall/sysret round-trip OK`.

### Результат в QEMU (run 40084859952, ISO из CI)
```text
SYSCALL/SYSRET enabled (EFER.SCE, STAR/LSTAR/FMASK)
ИрНа kernel: M1 subsystems OK (bootinfo/PMM/GDT/IDT/PIC/PIT/heap/LFB/syscall)
==================================================
ring 3: iretq в user mode, код по адресу 0x400000, стек 0x403000
Привет из ring 3! syscall/sysret OK
ring 3: syscall/sysret round-trip OK
keyboard echo: нажмите клавиши (serial + framebuffer)
key: 'a'
```
Скриншот подтверждает: сообщение из ring 3 напечатано в LFB, кириллица
корректна, после `sys_exit` ядро продолжило работу и обрабатывает IRQ1.

### Корневая причина #PF при первом входе в ring 3
Первый прогон дал `EXCEPTION 0xe #PF error=0x5 rip=0x400000 cs=0x23`.
Код 0x5 = present + user + read (выборка инструкции): страница присутствует,
но доступ из ring 3 запрещён. Причина — флаг U/S выставлялся только в
PT-записи, а x86 требует U/S=1 **на каждом уровне** трансляции (PML4 → PDPT →
PD → PT); иначе доступ отклоняется на верхнем уровне. Исправлено в
`map_user_4k`: U/S добавляется в PML4[0], запись PDPT и PD. Это безопасно:
итоговое право — конъюнкция уровней, поэтому identity-страницы в расщеплённой
таблице остаются supervisor-only (в их PT-записях U/S нет).

### Ограничения M1 (осознанные)
- Вход SYSCALL использует статический стек и статическую переменную для
  user RSP → вложенные syscall из ring 0 не поддерживаются. В M2 будет
  per-CPU GS-база + `swapgs`, как в Linux.
- `write` читает пользовательский буфер напрямую: SMAP выключен, проверки
  диапазона нет (кроме `len <= 4096`). Копирование `copy_from_user` — в M2.
- Тестовая user-программа — машинный код в `user.rs`, а не ELF. ELF-загрузчик
  и адресные пространства процессов — M2.
- Страница кода после записи переводится в RO, но остаётся исполняемой
  (NXE не включён).

### Коммиты
- `fab6a99` — `kernel: syscall/sysret, MSR STAR/LSTAR/FMASK, smoke-тест ring 3 (write/exit)`.
- `9b1c968` — `ci: проверки SYSCALL/SYSRET и возврата из ring 3`.
- `5bd59c4` — `kernel: paging — U/S на всех уровнях таблицы для user-страниц`.

### Дальше
Кооперативный планировщик (переключение контекста на одном ядре), затем
вытесняющий по PIT; после — ELF-загрузчик и адресные пространства (M2).

---

## Правила ведения файла
- После каждого завершённого этапа добавлять новый раздел с датой.
- Фиксировать корневые причины ошибок (не обходки), артефакты, хеш-подтверждения (checksum, rc) и коммиты.

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

## 2026-10-23/24 — Этап M1: базовое ядро (в работе, один дефект открыт)

### Что сделано и работает
ISO пересобран, ядро загружается в QEMU через GRUB Multiboot2 и проходит
конвейер инициализации M1:

```text
Booting `IrNa OS (M1)'
==================================================
ИрНа kernel: M1 boot start
PMM frames total = 32768        ← физический аллокатор (fallback 128 МиБ)
PMM frames free  = 32235
PMM alloc_frame OK              ← alloc/free кадра работают
GDT+TSS loaded (RSP0, IST1)     ← GDT + TSS через gdt_reload (NASM)
kernel heap alloc OK (100 bytes)← bump-пул 1 МиБ, global_allocator
VGA text console OK             ← VGA 0xB8000; кириллица пока не работает (CP437)
ИрНа kernel: M1 subsystems OK (bootinfo/PMM/GDT/heap/VGA)
```

Проверки: `./ci/run-qemu.sh check` → SUCCESS (rc=0);
`grub-file --is-x86-multiboot2 build/irna-kernel.elf` → rc=0;
`build/irna.iso` ≈ 4.9M.

### Созданные/изменённые файлы
- `kernel/src/arch.asm` — trampoline: сохранение EBX (MBI), карта памяти
  0–1 ГиБ (huge pages, MMIO read-only), экспорт `gdt_reload`, стеки для
  TSS.RSP0/IST1, information-request тег (`header_length=36`).
- `kernel/src/bootinfo.rs` — парсер MBI (memory map tag=6).
- `kernel/src/pmm.rs` — bitmap-аллокатор кадров.
- `kernel/src/gdt.rs` — GDT+TSS (user-селекторы заложены под M2 syscall).
- `kernel/src/heap.rs` — kmalloc-куча как `#[global_allocator]`.
- `kernel/src/vga.rs` — текстовая консоль 80×25, UTF-8 → CP437.
- `kernel/src/main.rs` — конвейер инициализации M1 + smoke-тесты.
- `kernel/linker.ld` — метка `_end` для PMM.
- `ci/build.sh` — `-Zbuild-std=core,alloc,panic_abort`, menuentry «IrNa OS (M1)».
- `ci/run-qemu.sh` — проверка строки «M1 subsystems OK».

### Открытый дефект: MBI memory map
`MBI ptr saved = 2191352`, дубль EBX совпадает, но по этому адресу читается
`magic = 1000` вместо `0x36D76289`. Ядро стабильно, PMM использует
детерминированный fallback 128 МиБ, поэтому smoke-тесты проходят.

Проверенные гипотезы:
- EBX потерян trampoline'ом — **исключено** (дубль `mb_mboot_ebx` совпадает).
- Отсутствие information-request тега — тег type=0 добавлен, заголовок принят
  GRUB (`grub-file` OK), но magic по-прежнему 1000.
- Последний дамп 8 слов по EBX был запущен, но не успел вывести результат —
  это первая точка продолжения.

Правдоподобный остаточный корень: адрес MBI лежит выше 1 МиБ и попадает в
зону, которую нужно читать до включения paging, либо EBX указывает не на
начало структуры (значение 1000 похоже на смещение внутри таблицы).

План:
1. Прогнать `./ci/build.sh --fast` + dbg-ISO и прочитать дамп слов MBI
   (`MBI word = ...`), определить, что реально лежит по EBX.
2. По результату: сдвинуть разбор или скопировать MBI в статический буфер
   в 32-битном trampoline до включения paging.
3. Включить реальный memory map в PMM и убрать fallback.

Дефект не блокирует M1, но должен быть закрыт до M2, где загрузочные модули
и точная карта памяти обязательны.

### Открытый дефект: кириллица в VGA
В VGA text mode кириллица отображается «крякозябрами». Причина: ROM-шрифт VGA
(CP437) не содержит кириллических глифов, а таблица в `vga.rs` отображает
буквы в коды псевдографики/латиницы. Serial-вывод при этом корректный (UTF-8).
Решение: свой 8×16 шрифт с кириллицей, загружаемый в VGA plane 2, либо
временная ASCII-заглушка в VGA.

### Дальше
Закрыть MBI-дефект и кириллицу в VGA, затем IDT + PIC remap, PIT-таймер,
PS/2 клавиатура, framebuffer/syscall.

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

### CI и тест
- GitHub Actions workflow отработал успешно (run `40062432124`).
- Артефакт `irna-iso` скачан: `C:\Projects\OS\artifacts\irna.iso` ≈ 11.9 МБ.
- ISO проверен в QEMU на ноутбуке: `M1 subsystems OK`, дефект MBI сохраняется.
- Рабочий цикл: push → Actions → artifact → QEMU.

### Дальше
M1: закрыть MBI-дефект, затем IDT/PIC, PIT, PS/2, framebuffer, syscall, планировщик.

---

## Правила ведения файла
- После каждого завершённого этапа добавлять новый раздел с датой.
- Фиксировать корневые причины ошибок (не обходки), артефакты, хеш-подтверждения (checksum, rc) и коммиты.

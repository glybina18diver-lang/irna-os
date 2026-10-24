# STATE — текущее состояние проекта «ИрНа»

> Обновлять в конце каждой сессии. Это краткая выжимка; подробности — в `HISTORY.md`.

**Дата:** 2026-10-24
**Этап:** M1 (в работе)
**Ветка:** `main`
**Последний коммит:** `21ace7e` — CI + документация + состояние M1

## Что сделано

### M0 — тулчейн и загрузка
- Multiboot2-заголовок, переход real → protected → long mode.
- GDT, identity-map, загрузка через GRUB (BIOS).
- Rust `no_std`-ядро, вывод в COM1.
- Скрипты `ci/build.sh`, `ci/run-qemu.sh`.
- Проверено в QEMU: `M0 hello-world (serial console) OK`.

### M1 — базовое ядро (конвейер работает, один дефект открыт)
- `kernel/src/bootinfo.rs` — парсер MBI (memory map tag=6).
- `kernel/src/pmm.rs` — bitmap-аллокатор кадров.
- `kernel/src/gdt.rs` — GDT + TSS (RSP0, IST1).
- `kernel/src/heap.rs` — bump-аллокатор на 1 МиБ, `#[global_allocator]`.
- `kernel/src/vga.rs` — текстовая консоль 80×25, UTF-8 → CP437 (кириллица).
- `kernel/src/arch.asm` — trampoline, identity map 0..1 ГиБ (huge pages,
  MMIO read-only), `gdt_reload`, стеки TSS.RSP0/IST1, information-request тег.
- `kernel/src/main.rs` — конвейер инициализации M1 + smoke-тесты.
- `kernel/linker.ld` — метка `_end` для PMM.

Фактический serial-вывод QEMU:

```text
Booting `IrNa OS (M1)'
ИрНа kernel: M1 boot start
PMM frames total = 32768        ← fallback 128 МиБ
PMM frames free  = 32235
PMM alloc_frame OK
GDT+TSS loaded (RSP0, IST1)
kernel heap alloc OK (100 bytes)
VGA text console OK
ИрНа kernel: M1 subsystems OK (bootinfo/PMM/GDT/heap/VGA)
```

`./ci/run-qemu.sh check` → SUCCESS, `grub-file --is-x86-multiboot2` → rc=0,
`build/irna.iso` ≈ 4.9M.

### Восстановление репозитория (2026-10-24)
- Сравнены `workspace_last` (M1) и `irna-os-main` (M0).
- Подтверждено: M0-снимок полностью входит в M1-рабочую область.
- Создан чистый репозиторий `C:\Projects\OS\irna-os` (31 файл).
- Подключён GitHub, выполнен push, автор — `Kay`.
- Удалён ошибочный `workspace2.zip`, `LICENSE` обновлён на `Copyright (c) 2026 Kay`.

### CI и тест (2026-10-24)
- GitHub Actions workflow собран успешно (run `40062432124`).
- Артефакт `irna-iso` скачан, `irna.iso` ≈ 11.9 МБ.
- Проверка в QEMU на ноутбуке: `M1 subsystems OK`; дефект MBI сохраняется.
- Рабочий цикл: push → Actions → artifact → QEMU.

## Следующие шаги (M1)

0. **Закрыть MBI-дефект**: дамп 8 слов по EBX, при необходимости скопировать
   MBI в статический буфер в 32-битном trampoline до включения paging,
   затем включить реальный memory map в PMM и убрать fallback.
1. IDT + обработчики исключений (page fault, double fault, GP).
2. PIC (8259) + PIT (таймер).
3. PS/2 клавиатура (scancode set 2, EN/RU).
4. Framebuffer / LFB (после VGA text).
5. `syscall`/`sysret` + минимальный libc.
6. Кооперативный, затем вытесняющий планировщик.

## Известные проблемы и ограничения

- GRUB 2.06 (Debian/Ubuntu) не принимает request-теги Multiboot2
  (framebuffer/console) → используется минимальный заголовок.
- **MBI memory map не разбирается** (`MBI found = 0`). PMM работает через
  детерминированный fallback 128 МиБ, smoke-тесты проходят.
  Диагностика: `MBI ptr saved = 2191352`, дубль EBX совпадает, но по адресу
  читается `magic = 1000` вместо `0x36D76289`.
  Проверено/исключено: EBX не теряется; information-request тег (type=0)
  добавлен (`header_length=36`, `grub-file` OK), но magic всё равно 1000.
  Гипотеза: MBI лежит выше 1 МиБ / в зоне, которую нужно читать до paging,
  либо EBX указывает не на начало структуры.
  План: дамп 8 слов по EBX (`MBI word = ...`), при необходимости — копировать
  MBI в статический буфер в 32-битном trampoline до включения paging,
  затем включить реальный memory map в PMM и убрать fallback.
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

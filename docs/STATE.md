# STATE — текущее состояние проекта «ИрНа»

> Обновлять в конце каждой сессии. Это краткая выжимка; подробности — в `HISTORY.md`.

**Дата:** 2026-10-24
**Этап:** M1 (в работе)
**Ветка:** `main`
**Последний коммит:** `2d9b88e` — восстановленная полная кодовая база

## Что сделано

### M0 — тулчейн и загрузка
- Multiboot2-заголовок, переход real → protected → long mode.
- GDT, identity-map, загрузка через GRUB (BIOS).
- Rust `no_std`-ядро, вывод в COM1.
- Скрипты `ci/build.sh`, `ci/run-qemu.sh`.
- Проверено в QEMU: `M0 hello-world (serial console) OK`.

### M1 — базовое ядро (частично)
- `kernel/src/bootinfo.rs` — разбор Multiboot2 memory map.
- `kernel/src/pmm.rs` — физический аллокатор кадров.
- `kernel/src/gdt.rs` — GDT + TSS (RSP0, IST1).
- `kernel/src/heap.rs` — bump-аллокатор на 1 МиБ.
- `kernel/src/vga.rs` — текстовый VGA-консоль.
- `kernel/src/arch.asm` — paging (identity map 0..1 ГиБ), передача MBI.
- Проверено в QEMU: `M1 subsystems OK (bootinfo/PMM/GDT/heap/VGA)`.

### Восстановление репозитория (2026-10-24)
- Сравнены `workspace_last` (M1) и `irna-os-main` (M0).
- Подтверждено: M0-снимок полностью входит в M1-рабочую область.
- Создан чистый репозиторий `C:\Projects\OS\irna-os` (31 файл).
- Подключён GitHub, выполнен push, автор — `Kay`.
- Удалён ошибочный `workspace2.zip`, `LICENSE` обновлён на `Copyright (c) 2026 Kay`.

## Следующие шаги (M1)

1. IDT + обработчики исключений (page fault, double fault, GP).
2. PIC (8259) + PIT (таймер).
3. PS/2 клавиатура (scancode set 2, EN/RU).
4. Framebuffer / LFB (после VGA text).
5. `syscall`/`sysret` + минимальный libc.
6. Кооперативный, затем вытесняющий планировщик.

## Известные проблемы и ограничения

- GRUB 2.06 (Debian/Ubuntu) не принимает request-теги Multiboot2
  (framebuffer/console) → используется минимальный заголовок.
- В текущем ISO MBI memory map не находится (`MBI found = 0`), PMM работает
  через fallback. Причина — минимальный MB2-заголовок без information request.
  Вернуться к этому в M1.
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

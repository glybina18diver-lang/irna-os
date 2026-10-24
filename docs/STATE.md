# STATE — текущее состояние проекта «ИрНа»

> Обновлять в конце каждой сессии. Это краткая выжимка; подробности — в `HISTORY.md`.

**Дата:** 2026-10-24
**Этап:** M1 (в работе)
**Ветка:** `main`
**Последний коммит:** `fc5fa6a` — исправление MBI (копия до paging, magic из EAX)

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
- `kernel/src/heap.rs` — bump-аллокатор на 1 МиБ, `#[global_allocator]`.
- `kernel/src/vga.rs` — текстовая консоль 80×25, UTF-8 → CP437.
  Кириллица в ROM-шрифте VGA отсутствует — нужен свой 8×16 шрифт.
- `kernel/src/arch.asm` — trampoline, identity map 0..1 ГиБ (huge pages,
  MMIO read-only), `gdt_reload`, стеки TSS.RSP0/IST1, information-request тег
  (type=1, padding до 8 байт), копия MBI в `.mbi_copy` до включения paging.
- `kernel/src/main.rs` — конвейер инициализации M1 + smoke-тесты.
- `kernel/linker.ld` — метка `_end` для PMM, секция `.mbi_copy`.

Фактический serial-вывод QEMU (после исправления MBI):

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
VGA text console OK
ИрНа kernel: M1 subsystems OK (bootinfo/PMM/GDT/heap/VGA)
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

1. IDT + обработчики исключений (page fault, double fault, GP).
2. PIC (8259) + PIT (таймер).
3. PS/2 клавиатура (scancode set 2, EN/RU).
4. Framebuffer / LFB (после VGA text).
5. `syscall`/`sysret` + минимальный libc.
6. Кооперативный, затем вытесняющий планировщик.

## Известные проблемы и ограничения

- GRUB 2.06 (Debian/Ubuntu) не принимает request-теги framebuffer (0x14),
  поэтому framebuffer-тег не добавляем; information request (type=1) работает.
- PMM сохраняет детерминированный fallback 128 МиБ на случай, если MBI не
  разобран; в QEMU теперь используется реальная карта памяти.
- **VGA text: кириллица — «крякозябры».** ROM-шрифт VGA (CP437) не содержит
  кириллицу; текущая таблица в `vga.rs` отображает её в символы псевдографики.
  Serial-вывод при этом корректный (UTF-8). Нужен свой 8×16 шрифт с кириллицей
  или временная ASCII-заглушка в VGA.
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

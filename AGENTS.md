# AGENTS.md — инструкции для ИИ-агентов

Проект: **«ИрНа» (irna-os)** — настольная ОС для x86-64, пишется с нуля.
Репозиторий: `C:\Projects\OS\irna-os`
Remote: `https://github.com/glybina18diver-lang/irna-os.git` (ветка `main`)
Лицензия: MIT.

## Стек

- Ядро: Rust `no_std`, x86-64, Multiboot2 + GRUB (BIOS).
- GUI (позже): C++ / Qt6, собственный window server.
- Python в ядре и системных сервисах не используется.

## Текущий этап

**M1 в работе.** Сделано: Multiboot2 bootinfo, PMM, GDT+TSS, heap, IDT/PIC,
PIT 100 Гц, PS/2 клавиатура, LFB-консоль с кириллицей (PT Mono 10×16).
Дальше: `syscall`/`sysret` + libc-mini, планировщик, раскладка EN/RU.
Актуальное состояние — `docs/STATE.md`.

## Перед началом работы прочитать

1. `docs/STATE.md` — где мы сейчас.
2. `HISTORY.md` — что уже сделано и какие были проблемы.
3. `ROADMAP.md` — план этапов.
4. `DECISIONS.md` — архитектурные решения.
5. `CONTEXT.md`, `SPEC.md` — по необходимости.

## Правила

- Имя ОС не хардкодить: `branding.md`, `branding/name.h`, `kernel/src/brand.rs`,
  `bootloader/config.toml`.
- Архитектурные решения записывать в `DECISIONS.md` (нумерация D-NNN).
- После каждой сессии обновлять `HISTORY.md` и `docs/STATE.md`.
- Коммиты с префиксом компонента: `kernel: ...`, `docs: ...`, `ci: ...`, `build: ...`.
- Не коммитить `build/`, `kernel/target/`, `*.elf`, `*.o`, `artifacts/`.
- Никакого кода Microsoft и проприетарных ассетов. Стороннее — только
  MIT/Apache/BSD/OFL/LGPL (динамическая линковка).
- Не удалять чужую историю и файлы без явной необходимости.

## Сборка и тест

- Полная сборка ISO: GitHub Actions (`.github/workflows/build.yml`),
  артефакт `irna-iso`.
- Быстрая локальная проверка: `cd kernel && cargo +nightly build --release
  --target x86_64-unknown-none -Zbuild-std=core,alloc,panic_abort`
  (нужны Rust nightly + `rust-src` + NASM).
- Тест ISO: QEMU на Windows (см. `docs/BUILD.md`).
- Полные инструкции: `docs/BUILD.md`.

## Окружение (ноутбук)

- Windows 11, 4 ГБ RAM, Celeron N2840.
- Портативный Git: `C:\Projects\OS\tools\MinGit\cmd\git.exe`
- GitHub CLI: `C:\Projects\OS\tools\gh\bin\gh.exe`
- QEMU: `C:\Projects\OS\tools\qemu\qemu-system-x86_64.exe`
- WSL2 и виртуальные машины на ноутбуке не использовать.
- Тяжёлую сборку делать в CI; локально — только лёгкие проверки и QEMU.

## Контекст между чатами

Новый чат начинать по шаблону `docs/HANDOFF.md`.

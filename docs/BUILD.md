# BUILD — сборка и тестирование «ИрНа»

## 1. Рекомендуемая схема

- **Полная сборка ISO** — GitHub Actions (Ubuntu в облаке).
- **Тест** — QEMU на Windows (ноутбук), без виртуализации.
- **Локальная быстрая проверка** — Rust + NASM (опционально, только `cargo build` ELF).

Так ноутбук с 4 ГБ RAM не занят тяжёлым тулчейном.

## 2. Сборка в GitHub Actions

Workflow: `.github/workflows/build.yml`.
Запускается на `push` в `main`, на pull request и вручную (`workflow_dispatch`).

Что делает:
1. Ставит `nasm`, `grub-pc-bin`, `grub-common`, `xorriso`, `mtools`.
2. Ставит Rust nightly + `rust-src`.
3. Запускает `ci/build.sh`.
4. Выкладывает артефакты `irna-iso` (`build/irna.iso`) и `irna-kernel-elf`.

### Скачать ISO

Через GitHub CLI:

```powershell
$gh = "C:\Projects\OS\tools\gh\bin\gh.exe"
& $gh run list --limit 5
& $gh run download <RUN_ID> -n irna-iso -D C:\Projects\OS\artifacts
```

Или в браузере: вкладка **Actions** → нужный запуск → **Artifacts** → `irna-iso`.

## 3. Тест в QEMU (Windows)

QEMU может лежать в одном из двух мест (скрипт проверяет их по порядку):

```text
C:\Program Files\qemu\qemu-system-x86_64.exe      # основной путь
C:\Projects\OS\tools\qemu\qemu-system-x86_64.exe  # портативный fallback
```

### Рекомендуемый способ — `ci/run-qemu.ps1`

```powershell
cd C:\Projects\OS\irna-os
powershell -NoProfile -ExecutionPolicy Bypass -File ci\run-qemu.ps1 `
  -Iso C:\Projects\OS\artifacts\irna.iso
```

Скрипт запускает QEMU без окна (`-display none`, `-m 512`, `-no-reboot`),
serial идёт через TCP, клавиша `a` отправляется через QMP после появления
`keyboard echo:`, проверяются все маркеры M1. Код возврата `0` — успех.
Флаги и логи описаны в `docs/QUICKSTART.md` §5.

### С окном VGA

```powershell
$qemu = "C:\Program Files\qemu\qemu-system-x86_64.exe"
& $qemu `
  -cdrom "C:\Projects\OS\artifacts\irna.iso" `
  -m 512 `
  -serial stdio
```

Выход из QEMU: `Ctrl+Alt+Q` или закрыть окно.

### Без окна, serial в файл

```powershell
$qemu = "C:\Program Files\qemu\qemu-system-x86_64.exe"
& $qemu `
  -cdrom "C:\Projects\OS\artifacts\irna.iso" `
  -m 512 `
  -display none `
  -serial file:"C:\Projects\OS\artifacts\serial.log" `
  -no-reboot
```

Проверка результата:

```powershell
Select-String -Path "C:\Projects\OS\artifacts\serial.log" -Pattern "M1 subsystems OK"
```

Примечание: `-serial file:` буферизует вывод; для интерактивной проверки
клавиатуры используйте `ci/run-qemu.ps1` (TCP + QMP).

### Если QEMU тормозит

- Оставить `-m 512`; при нехватке RAM — `-m 256` (PMM всё равно ограничен
  128 МиБ, см. `docs/TECH-DEBT.md` TD-001).
- Закрыть браузер и лишние приложения.
- Использовать `-display none` + serial-лог.
- При необходимости: `-accel tcg,thread=multi`.

## 4. Локальная быстрая проверка (опционально)

Нужны Rust nightly, `rust-src` и NASM.

```powershell
winget install Rustlang.Rustup
winget install NASM.NASM

rustup toolchain install nightly
rustup component add rust-src --toolchain nightly
```

Сборка ELF (без ISO):

```powershell
cd C:\Projects\OS\irna-os\kernel
cargo +nightly build --release --target x86_64-unknown-none `
  -Zbuild-std=core,alloc,panic_abort
```

Результат: `kernel\target\x86_64-unknown-none\release\irna-kernel` (ELF).
ISO на Windows не собирается — для этого нужен Linux/CI.

## 5. Полная сборка на Linux (если понадобится)

```bash
sudo apt-get update
sudo apt-get install -y --no-install-recommends \
  nasm grub-pc-bin grub-common xorriso mtools qemu-system-x86
rustup toolchain install nightly
rustup component add rust-src --toolchain nightly
./ci/build.sh
./ci/run-qemu.sh check
```

## 6. Частые проблемы

Полная таблица (включая `gh`, артефакты и PowerShell) — в
`docs/QUICKSTART.md` §9. Известные ограничения с планами — в
`docs/TECH-DEBT.md`.

| Симптом | Причина / решение |
|---|---|
| `не найден nasm` | Установить NASM и добавить в `PATH`. |
| `-Zbuild-std` не работает | Нужен nightly + `rust-src`; stable — только с `RUSTC_BOOTSTRAP=1`. |
| `grub-mkrescue` падает | Нужны `grub-pc-bin`, `xorriso`, `mtools`; в CI они ставятся. |
| QEMU не найден | Проверить `C:\Program Files\qemu\...` и `C:\Projects\OS\tools\qemu\...`; скрипт ищет оба. |
| QEMU не видит ISO | Проверить путь к `irna.iso`; запускать из-под пользователя. |
| Нет строки `M1 subsystems OK` | Смотреть `serial.log`; возможно, ядро упало до инициализации serial. |
| Клавиша не проверяется | В headless HMP `sendkey` не доходит до PS/2; нужен QMP (используется в `ci/run-qemu.ps1`). |
| Нет кириллицы | Ядро в VGA text fallback: в CP437 нет русских букв; основной путь — LFB. |

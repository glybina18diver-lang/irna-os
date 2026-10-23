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

QEMU установлен портативно:

```
C:\Projects\OS\tools\qemu\qemu-system-x86_64.exe
```

### С окном VGA

```powershell
& "C:\Projects\OS\tools\qemu\qemu-system-x86_64.exe" `
  -cdrom "C:\Projects\OS\artifacts\irna.iso" `
  -m 256M `
  -serial stdio
```

Выход из QEMU: `Ctrl+Alt+Q` или закрыть окно.

### Без окна, serial в файл

```powershell
& "C:\Projects\OS\tools\qemu\qemu-system-x86_64.exe" `
  -cdrom "C:\Projects\OS\artifacts\irna.iso" `
  -m 256M `
  -display none `
  -serial file:"C:\Projects\OS\artifacts\serial.log" `
  -no-reboot
```

Проверка результата:

```powershell
Select-String -Path "C:\Projects\OS\artifacts\serial.log" -Pattern "M1 subsystems OK"
```

### Если QEMU тормозит

- Оставить `-m 256M` (не больше 512M).
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
sudo apt-get install -y nasm grub-pc-bin grub-common xorriso mtools
rustup toolchain install nightly
rustup component add rust-src --toolchain nightly
./ci/build.sh
./ci/run-qemu.sh check
```

## 6. Частые проблемы

| Симптом | Причина / решение |
|---|---|
| `не найден nasm` | Установить NASM и добавить в `PATH`. |
| `-Zbuild-std` не работает | Нужен nightly + `rust-src`; stable — только с `RUSTC_BOOTSTRAP=1`. |
| `grub-mkrescue` падает | Нужны `grub-pc-bin`, `xorriso`, `mtools`; в CI они ставятся. |
| QEMU не видит ISO | Проверить путь к `irna.iso`; запускать из-под пользователя. |
| Нет строки `M1 subsystems OK` | Смотреть `serial.log`; возможно, ядро упало до инициализации serial. |

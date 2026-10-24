# QUICKSTART — «ИрНа» с нуля на новой машине

> Для нового разработчика и для случая «начинаю на другом компьютере».
> Полный цикл: клонирование → сборка в GitHub Actions → скачивание ISO →
> проверка в QEMU. Все команды проверены на Windows 11 x64 (4 ГБ RAM,
> Celeron N2840) с QEMU 11.1.0.
>
> Если что-то не сходится — сначала `docs/BUILD.md`, затем
> `docs/TECH-DEBT.md` (там же известные ограничения).

## 0. Что уже работает (M1)

- Загрузка через GRUB (BIOS, Multiboot2), разбор memory map.
- PMM (bitmap), GDT + TSS, IDT (48 гейтов), PIC 0x20..0x2F, PIT 100 Гц.
- PS/2 клавиатура (IRQ1, scancode set 1), раскладка US.
- LFB-консоль 24/32 bpp, шрифт PT Mono 10×16 с кириллицей.
- SYSCALL/SYSRET, ring 3, `write`/`exit`.
- Кооперативный планировщик: 8 слотов, round-robin, 3 демо-задачи.

Чего ещё нет: вытеснения, раскладки RU, ELF-загрузчика, ФС, GUI, сети.
Актуальный список — `ROADMAP.md` и `docs/STATE.md`.

## 1. Что нужно на машине

| Компонент | Обязательно | Где взять / путь |
|---|---|---|
| Windows 10/11 x64 | да (для теста) | — |
| PowerShell 5.1+ | да | встроен |
| Git | да | `C:\Projects\OS\tools\MinGit\cmd\git.exe` |
| GitHub CLI | да (для артефактов) | `C:\Projects\OS\tools\gh\bin\gh.exe` |
| QEMU | да (для теста) | `C:\Program Files\qemu\qemu-system-x86_64.exe` или `C:\Projects\OS\tools\qemu\qemu-system-x86_64.exe` |
| Rust nightly + `rust-src` | нет* | только для локальной сборки ELF |
| NASM | нет* | только для локальной сборки ELF |
| Linux/Ubuntu | нет** | только для полной сборки ISO |

\* Стандартный цикл: собирает CI, локально только QEMU.
\** На ноутбуке проекта WSL2 и виртуальные машины не использовать — не хватает
RAM. ISO собирается в GitHub Actions.

Минимум: ~2 ГБ свободного диска под репозиторий, артефакты и логи.

## 2. Клонирование

```powershell
$git = "C:\Projects\OS\tools\MinGit\cmd\git.exe"
cd C:\Projects\OS
& $git clone https://github.com/glybina18diver-lang/irna-os.git
cd C:\Projects\OS\irna-os

& $git config user.name "Kay"
& $git config user.email "ваш-email@example.com"
& $git log --oneline -5
```

Репозиторий публичный: для `clone` токен не нужен, для `push` — нужен
авторизованный GitHub CLI или PAT.

Проверка авторизации:

```powershell
$gh = "C:\Projects\OS\tools\gh\bin\gh.exe"
& $gh auth status
```

Если не авторизован:

```powershell
& $gh auth login
```

## 3. Рабочий цикл (рекомендуемый)

Локально нет Rust/NASM, поэтому любая правка ядра проверяется через CI:

1. Внести изменения.
2. Закоммитить с префиксом компонента.
3. Отправить в `main`.
4. Дождаться зелёного workflow.
5. Скачать `irna-iso`.
6. Запустить `ci/run-qemu.ps1`.

```powershell
cd C:\Projects\OS\irna-os
& $git add -A
& $git commit -m "kernel: короткое описание"
& $git push

& $gh run list --limit 3
& $gh run watch <RUN_ID> --exit-status
& $gh run download <RUN_ID> -n irna-iso -D C:\Projects\OS\artifacts
```

Важно:

- `git push` пишет прогресс в **stderr**. PowerShell может показать
  «Exited with code 1», хотя push прошёл. Проверяйте
  `& $git log origin/main -1` или `& $git status`.
- `gh run watch` требует **ID запуска**, а не URL. ID берётся из
  `gh run list`.
- Артефакты GitHub хранятся ограниченное время; если скачивание не удалось —
  перезапустите workflow (`gh workflow run build.yml`).

## 4. Скачивание ISO

Через CLI (одна команда на артефакт):

```powershell
$gh = "C:\Projects\OS\tools\gh\bin\gh.exe"
& $gh run download <RUN_ID> -n irna-iso        -D C:\Projects\OS\artifacts
& $gh run download <RUN_ID> -n irna-kernel-elf -D C:\Projects\OS\artifacts\elf
```

После этого:

- `C:\Projects\OS\artifacts\irna.iso` — загрузочный образ (~12 МБ);
- `C:\Projects\OS\artifacts\elf\irna-kernel.elf` — ядро для `grub-file`/отладки.

Через браузер: вкладка **Actions** → нужный run → **Artifacts** →
`irna-iso`.

## 5. Запуск и проверка в QEMU (Windows)

### Рекомендуемый способ — скрипт

```powershell
cd C:\Projects\OS\irna-os
powershell -NoProfile -ExecutionPolicy Bypass -File ci\run-qemu.ps1 `
  -Iso C:\Projects\OS\artifacts\irna.iso
```

Что делает скрипт:

- запускает QEMU **без окна** (`-display none`, `-m 512`, `-no-reboot`);
- serial — через TCP `127.0.0.1:4600`, QMP — `127.0.0.1:4601`;
- ждёт `keyboard echo:` и отправляет клавишу `a` через QMP
  (`input-send-event`);
- проверяет маркеры и возвращает `0`, только если все найдены.

Параметры:

| Параметр | По умолчанию | Зачем |
|---|---|---|
| `-Iso` | `artifacts\irna.iso` | путь к образу |
| `-Seconds` | `30` | на слабом CPU увеличьте до `45` |
| `-Key` | `a` | какая клавиша проверяется; `""` — без проверки |
| `-Screenshot` | пусто | путь к PNG; включает QMP-скриншот |
| `-ScreenshotAt` | `0` | секунда скриншота, если `-Key ""` |
| `-Log` | `artifacts\logs\qemu-serial.log` | основной лог |
| `-SerialPort` / `-QmpPort` | `4600` / `4601` | если порты заняты |

Примеры:

```powershell
# без клавиатуры, 45 секунд
powershell -NoProfile -ExecutionPolicy Bypass -File ci\run-qemu.ps1 `
  -Iso C:\Projects\OS\artifacts\irna.iso -Key "" -Seconds 45

# со скриншотом экрана
powershell -NoProfile -ExecutionPolicy Bypass -File ci\run-qemu.ps1 `
  -Iso C:\Projects\OS\artifacts\irna.iso `
  -Screenshot C:\Projects\OS\artifacts\screen.png
```

Логи после прогона:

- `artifacts\logs\qemu-serial.log` — сырой serial (UTF-8);
- `artifacts\logs\qemu-serial.clean.log` — без ANSI-последовательностей;
- `artifacts\logs\qemu-serial.qemu-err.log` — stderr QEMU.

### Ожидаемый порядок маркеров

```text
MBI found        = 1
GDT+TSS loaded (RSP0, IST1)
IDT loaded (48 gates: 32 exception + 16 IRQ)
IDT int3 smoke OK
PIC remapped (IRQ 0x20..0x2F)
PIT programmed (100 Hz)
PIT: 20 ticks OK
PS/2 keyboard ready (IRQ1)
LFB ... 800x600 / 24 bpp
framebuffer console OK (800x600, 80x37 chars)
SYSCALL/SYSRET enabled
task A/B/C: tick ...
sched: tasks done = 3
M1 subsystems OK (...)
ring 3: iretq в user mode ...
Привет из ring 3! syscall/sysret OK
syscall/sysret round-trip OK
keyboard echo: ...
key: 'a'
```

Успех скрипта:

```text
OK: загрузка ядра
OK: MBI разобран
...
OK: клавиша 'a' по IRQ1
==> SUCCESS: ISO загрузился, прерывания и клавиатура работают.
```

### Вручную (если скрипт недоступен)

С окном:

```powershell
& "C:\Program Files\qemu\qemu-system-x86_64.exe" `
  -cdrom "C:\Projects\OS\artifacts\irna.iso" `
  -m 512 -serial stdio
```

Без окна, serial в файл:

```powershell
& "C:\Program Files\qemu\qemu-system-x86_64.exe" `
  -cdrom "C:\Projects\OS\artifacts\irna.iso" `
  -m 512 -display none -no-reboot `
  -serial file:"C:\Projects\OS\artifacts\serial.log"

Select-String -Path C:\Projects\OS\artifacts\serial.log -Pattern "M1 subsystems OK"
```

Учтите: `-serial file:` буферизует вывод, поэтому скрипт использует TCP.
Для интерактивной проверки клавиатуры нужен QMP (`input-send-event`), HMP
`sendkey` в headless-режиме до PS/2 не доходит.

## 6. Полная сборка на Linux (если понадобится)

```bash
sudo apt-get update
sudo apt-get install -y --no-install-recommends \
  nasm grub-pc-bin grub-common xorriso mtools qemu-system-x86

rustup toolchain install nightly
rustup component add rust-src --toolchain nightly

cd irna-os
chmod +x ci/build.sh ci/run-qemu.sh
./ci/build.sh          # build/irna-kernel.elf + build/irna.iso
./ci/run-qemu.sh check # headless-проверка, 20 секунд
```

Полезное:

- `./ci/build.sh --fast` — только ELF, без ISO (быстрая проверка компиляции);
- `./ci/run-qemu.sh interactive` — окно QEMU + serial в терминале;
- `./ci/run-qemu.sh efi` — требует OVMF; текущий ISO собирается BIOS-only,
  поэтому режим не является рабочим путём (см. TD-024).

`ci/build.sh` сам выставляет `RUSTC_BOOTSTRAP=1` и собирает через
`-Zbuild-std=core,alloc,panic_abort` в `build/cargo-target`.

## 7. Локальная проверка компиляции на Windows (опционально)

Нужны Rust nightly, `rust-src` и NASM (`kernel/build.rs` вызывает `nasm`).

```powershell
winget install Rustlang.Rustup
winget install NASM.NASM

rustup toolchain install nightly
rustup component add rust-src --toolchain nightly

cd C:\Projects\OS\irna-os\kernel
cargo +nightly build --release --target x86_64-unknown-none `
  -Zbuild-std=core,alloc,panic_abort
```

Результат: `kernel\target\x86_64-unknown-none\release\irna-kernel` (ELF).
ISO на Windows не собирается — для этого нужен Linux/CI.

## 8. Структура репозитория

| Путь | Что внутри |
|---|---|
| `kernel/src/` | ядро: `main.rs`, `arch.asm`, `bootinfo.rs`, `pmm.rs`, `paging.rs`, `heap.rs`, `gdt.rs`, `idt.rs`, `pic.rs`, `pit.rs`, `irq.rs`, `keyboard.rs`, `fb.rs`, `font.rs`, `vga.rs`, `console.rs`, `syscall.rs`, `user.rs`, `sched.rs`, `serial.rs` |
| `kernel/build.rs`, `kernel/linker.ld` | сборка `arch.asm` через NASM, карта линковки, метка `_end` |
| `ci/build.sh` | ELF + проверка Multiboot2 + ISO через `grub-mkrescue` |
| `ci/run-qemu.ps1` | headless-стенд Windows: serial TCP, QMP-клавиша, скриншот |
| `ci/run-qemu.sh` | headless-проверка на Linux (`check`/`interactive`/`efi`) |
| `.github/workflows/build.yml` | CI: сборка и артефакты `irna-iso`, `irna-kernel-elf` |
| `docs/` | `STATE.md`, `BUILD.md`, `TECH-DEBT.md`, `QUICKSTART.md`, `HANDOFF.md` |
| `tools/` | `gen-font8x16.ps1`, `preview-font8x16.ps1` |
| `assets/fonts/` | `OFL-PT-Mono.txt` (лицензия; TTF в репозитории нет) |
| `branding.md`, `kernel/src/brand.rs` | имя ОС в одном месте (D-008) |

## 9. Типовые проблемы

| Симптом | Причина / что делать |
|---|---|
| `нет ISO: ...` | Скачайте артефакт (`gh run download`) или укажите `-Iso`. |
| QEMU не найден | Установите QEMU или поправьте `$qemu` в `ci/run-qemu.ps1`. |
| `gh: not logged in` | `& $gh auth login`. |
| `gh run watch` без ID | Передайте ID из `gh run list`, URL не подходит. |
| Пустой serial-лог | Увеличьте `-Seconds` до 45; проверьте `*.qemu-err.log`; закройте браузер (мало RAM). |
| `FAIL: загрузка ядра` | ISO старый/битый; проверьте `M1 subsystems OK` в `.clean.log`. |
| `unsupported tag: 0x14` | Ошибка MB2-заголовка/тегов; см. HISTORY (M0, v4). |
| `-Zbuild-std` не работает | Нужен nightly + `rust-src`; stable — только с `RUSTC_BOOTSTRAP=1`. |
| `grub-mkrescue` падает | Нужны `grub-pc-bin`, `grub-common`, `xorriso`, `mtools`. |
| Нет кириллицы | Ядро ушло в VGA text fallback (нет LFB) — CP437 не содержит русских букв. |
| Клавиша не проверяется | В headless HMP `sendkey` не работает; скрипт использует QMP. Проверьте `-Key` и QMP-порт. |
| `.ps1` ломается на кириллице | PowerShell 5.1 читает файл как ANSI: сохраняйте `.ps1` в UTF-8 **с BOM**. |
| `git push`: «Exited with code 1» | Прогресс идёт в stderr; проверьте `git log origin/main -1`. |
| Артефакт не скачивается | Истёк срок хранения — перезапустите workflow. |

## 10. Правила проекта (кратко)

- Перед работой прочитать `AGENTS.md`, `docs/STATE.md`, `HISTORY.md`,
  `ROADMAP.md`, `DECISIONS.md`.
- Коммиты: `kernel: ...`, `docs: ...`, `ci: ...`, `build: ...`.
- Архитектурные решения — в `DECISIONS.md` (`D-NNN`).
- Не коммитить `build/`, `kernel/target/`, `artifacts/`, `*.elf`, `*.iso`.
- Сторонние ассеты — только MIT/Apache/BSD/OFL/LGPL.
- В конце сессии обновить `HISTORY.md`, `docs/STATE.md`, а при необходимости —
  `docs/TECH-DEBT.md` и `docs/QUICKSTART.md`.

## 11. Чек-лист «я на новой машине»

- [ ] Репозиторий склонирован, `git log` показывает `af65781` или новее.
- [ ] `gh auth status` — авторизован.
- [ ] QEMU найден по одному из двух путей.
- [ ] ISO скачан в `C:\Projects\OS\artifacts\irna.iso`.
- [ ] `ci\run-qemu.ps1` завершился с кодом `0` и `SUCCESS`.
- [ ] Прочитаны `docs/STATE.md` и `docs/TECH-DEBT.md`.
- [ ] Понятна первая задача (из `ROADMAP.md` / `TECH-DEBT.md`).

## 12. Правило обновления этого файла

`QUICKSTART.md` — такой же живой документ, как `HISTORY.md`. Обновляйте его,
когда:

- меняется путь/версия инструмента (Git, `gh`, QEMU, Rust);
- меняются флаги `ci/run-qemu.ps1` или `ci/build.sh`;
- меняется имя артефакта или состав ISO;
- добавляется/меняется обязательный маркер в serial;
- появляется новый поддерживаемый способ сборки или ОС;
- новый человек прошёл инструкцию и споткнулся — добавьте симптом в §9.

Требования к содержимому: команды должны копироваться и выполняться без
правок; пути — реальные для этого проекта; при изменении скрипта сначала
обновляется скрипт, затем этот файл и `docs/BUILD.md`. Раз в этап инструкция
проверяется на чистой машине.

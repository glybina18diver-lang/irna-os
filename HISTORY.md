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

## Настройки репозитория (24.10.2026)
- Автор коммитов и владелец репозитория: **glybina18diver-lang** `<glybina18diver-lang@users.noreply.github.com>`.
- Все исторические коммиты переписаны (`git filter-branch`), глобальный `git config` обновлён — все будущие коммиты автоматически пишутся от этого автора.

## Правила ведения файла
- После каждого завершённого этапа добавлять новый раздел с датой.
- Фиксировать корневые причины ошибок (не обходки), артефакты, хеш-подтверждения (checksum, rc) и коммиты.

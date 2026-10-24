# ROADMAP ОС «ИрНа» — статус этапов (единственный источник правды о прогрессе)

Легенда: [ ] не начато | [~] в работе | [x] готово

## M0 — Инструменты и hello-world  [x]
- [x] Linux-тулчейн (WSL2 Ubuntu или второй диск) + rustup + clang + cmake + Qt6 + qemu-system-x86_64
- [x] Скелет репозитория (структура из SPEC.md §7)
- [x] Ядро: print имени ОС (из branding-константы) в serial console из QEMU
- [x] Скрипты ci/build.sh, ci/run-qemu.sh

## M1 — Базовое ядро (Rust)  [~]
- [x] GDT/IDT, обработчики исключений, PIC  (IDT: 48 гейтов, PIC: 0x20..0x2F)
- [x] Физический аллокатор (frame allocator)
- [x] Виртуальная память (paging), heap + kmalloc  (identity map + bump-heap)
- [~] Таймер (PIT 100 Гц) + cooperative планировщик  (PIT готов; планировщик — нет)
- [~] Framebuffer-вывод (VGA text -> LFB)  (VGA text готов; LFB — нет)
- [x] PS/2 клавиатура (IRQ1, scancode set 1; раскладка EN/RU — после LFB)
- [ ] syscall/sysret + libc-mini (write/read/exit)
- [ ] Вытесняющий многозадачный планировщик

## M2 — Процессы и файлы  [ ]
- [ ] ELF-загрузчик user-space бинарников
- [ ] tmpfs + POSIX-like VFS (/bin /dev /home)
- [ ] init (PID 1) на Rust
- [ ] fork/execve/wait завершены
- [ ] Чтение ext2 (своя реализация) ИЛИ свой простой ФС

## M3 — Window Server  [ ]
- [ ] Решение: свой winserv (C++) vs wlroots — см. D-006, SPEC §9 вопрос 3
- [ ] IPC поверх unix socket, бинарный протокол (docs/winserv-protocol.md)
- [ ] Композитинг поверхностей в LFB
- [ ] Маршрутизация ввода, фокус, перетаскивание окон
- [ ] Демонстрационный C++ клиент: movable window

## M4 — Desktop Shell «как Windows 10»  [ ]
- [ ] QPA-плагин Qt под наш winserv
- [ ] Панель задач, кнопка «Пуск», меню с программами
- [ ] Менеджер окон: titlebar, snap, Alt+Tab
- [ ] Приложение «Блокнот» (Qt) работает в системе
- [ ] Обои, часы, трей

## M5 — Данные и медиа  [ ]
- [ ] AHCI-драйвер, запись на диск, persistence
- [ ] Медиаплеер: декод (ffmpeg LGPL) без звука
- [ ] HDA-аудио драйвер + звук в плеере
- [ ] Проводник файлов (упрощённый)

## M6+ — Экосистема  [ ]
- [ ] Сеть (e1000), DNS/HTTP-клиент
- [ ] Реальное железо (Lenovo A10-7800B)
- [ ] Настройки системы, обновления
- [ ] Решение по EXE/PE-эмуляции (пересмотр D-005)

---
Журнал изменений статуса:
- 2026-10-19: создана дорожная карта, этапы M0–M6+. Решения D-001…D-007 зафиксированы.
- 2026-10-23: M0 закрыт (сборка + QEMU check SUCCESS).
- 2026-10-24: репозиторий восстановлен; M1 в работе — готовы bootinfo, PMM, GDT+TSS, heap, VGA text.
- 2026-10-24: MBI-дефект закрыт (magic из EAX, копия MBI до paging); PMM использует реальную карту памяти.
- 2026-10-24: M1 — IDT (48 гейтов) + PIC remap 0x20..0x2F + PIT 100 Гц проверены в QEMU.
- 2026-10-24: M1 — PS/2 клавиатура (IRQ1, scancode set 1) проверена в QEMU (`key: 'a'`).

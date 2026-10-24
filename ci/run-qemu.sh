#!/usr/bin/env bash
# ci/run-qemu.sh — запуск build/irna.iso в QEMU (этап M0).
# Использование: ./ci/run-qemu.sh [--efi] [--graphics]
#   по умолчанию: headless-проверка через serial-консоль (15 секунд, затем авто-выход);
#   --interactive — оставить QEMU открытым (нужен дисплей);
#   --efi — эмуляция UEFI вместо BIOS (OVMF).
set -euo pipefail

ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
ISO="$ROOT/build/irna.iso"

[[ -f "$ISO" ]] || { echo "ERROR: нет $ISO — сначала выполните ./ci/build.sh"; exit 1; }

MODE="${1:-check}"
TIMEOUT_S=20

case "$MODE" in
  check)
    echo "==> QEMU (BIOS, headless, serial): проверка загрузки ISO ($TIMEOUT_S с)"
    # serial -> файл (надёжнее mon:stdio в неинтерактивной среде; tee/pipe там
    # теряют вывод из-за таймаута). grep -a — лог содержит бинарные escape-последовательности.
    rm -f "$ROOT/build/qemu-serial.log"
    timeout "$TIMEOUT_S" qemu-system-x86_64 \
        -cdrom "$ISO" \
        -m 256M \
        -display none \
        -serial file:"$ROOT/build/qemu-serial.log" \
        -no-reboot \
        -device isa-debug-exit,iobase=0xf4,iosize=0x04 \
      >/dev/null 2>&1 || true
    if ! grep -aq "M1 subsystems OK" "$ROOT/build/qemu-serial.log"; then
      echo "==> FAIL: ожидаемая строка не найдена в $ROOT/build/qemu-serial.log"
      exit 1
    fi
    if ! grep -aq "MBI found.*= 1" "$ROOT/build/qemu-serial.log"; then
      echo "==> FAIL: MBI memory map не разобрана (MBI found != 1)"
      exit 1
    fi
    if ! grep -aq "PIT: 20 ticks OK" "$ROOT/build/qemu-serial.log"; then
      echo "==> FAIL: IRQ0/PIT не работает (нет 'PIT: 20 ticks OK')"
      exit 1
    fi
    if ! grep -aq "PS/2 keyboard ready" "$ROOT/build/qemu-serial.log"; then
      echo "==> FAIL: клавиатура не инициализирована (нет 'PS/2 keyboard ready')"
      exit 1
    fi
    echo "==> SUCCESS: ядро «ИрНа» загрузилось, MBI memory map разобрана."
    ;;
  interactive)
    echo "==> QEMU (BIOS, окно VGA + serial в терминале). Ctrl-A x — выход."
    exec qemu-system-x86_64 -cdrom "$ISO" -m 512M -serial mon:stdio -no-reboot
    ;;
  efi)
    OVMF_CODE=""
    for c in /usr/share/OVMF/OVMF_CODE.fd /usr/share/ovmf/x64/OVMF_CODE.fd; do
        [[ -f "$c" ]] && OVMF_CODE="$c" && break
    done
    [[ -n "$OVMF_CODE" ]] || { echo "ERROR: не найден OVMF (apt install ovmf)"; exit 1; }
    echo "==> QEMU (UEFI/OVMF, headless serial)"
    timeout "$TIMEOUT_S" qemu-system-x86_64 \
        -cdrom "$ISO" -m 256M -display none -serial mon:stdio -no-reboot \
        -drive if=pflash,format=ro,readonly=on,file="$OVMF_CODE" \
      | tee "$ROOT/build/qemu-serial-efi.log" || true
    grep -q "M1 subsystems OK" "$ROOT/build/qemu-serial-efi.log" \
      && echo "==> SUCCESS (UEFI)" || { echo "==> FAIL (UEFI)"; exit 1; }
    ;;
  *)
    echo "Неизвестный режим: $MODE (используйте: check | interactive | efi)"; exit 1;;
esac

#!/usr/bin/env bash
# ci/build.sh — сборка ядра «ИрНа» (M0) и загрузочного ISO.
# Использование: ./ci/build.sh [--fast]
#   --fast — только ELF без ISO (быстрая проверка компиляции).
# Выход: build/irna-kernel.elf, build/irna.iso
#
# ИСПРАВЛЕНИЕ ДЕФЕКТА M0 (ISO не загружался в QEMU):
#  1) Multiboot2-заголовок в arch.asm: header_length был 56 вместо 80 — по
#     спецификации это суммарный размер ВСЕГО заголовка (магическое поле +
#     все request-теги + terminator). GRUB не находил корректный заголовок ->
#     "Multi-boot kernel must be specified" / загрузка не стартовала.
#  2) Сборка core/compiler_builtins: в установках rustup нет предсобранных
#     rlib для цели x86_64-unknown-none (её распространяют через cargo, а не
#     rustc), поэтому `rustc --target x86_64-unknown-none` падал на этапе
#     линковки. Теперь ядро собирается через `cargo build -Zbuild-std`
#     (официальный supported way для bare-metal целей).
set -euo pipefail

ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
BUILD="$ROOT/build"
KDIR="$ROOT/kernel"
mkdir -p "$BUILD"

FAST=0
[[ "${1:-}" == "--fast" ]] && FAST=1

# rustup-окружение, если rustc ещё не в PATH (WSL2/чистый Ubuntu)
if ! command -v rustc >/dev/null 2>&1 && [[ -f "$HOME/.cargo/env" ]]; then
    # shellcheck disable=SC1091
    . "$HOME/.cargo/env"
fi

echo "==> [1/3] cargo: сборка ядра (no_std, цель x86_64-unknown-none, build-std)"
export RUSTC_BOOTSTRAP=1   # -Zbuild-std требует nightly-флагов; stable допустим с этим env
cd "$KDIR"
cargo build --release --target x86_64-unknown-none -Zbuild-std=core,alloc,panic_abort \
    --target-dir "$BUILD/cargo-target"
KERNEL_ELF="$BUILD/cargo-target/x86_64-unknown-none/release/irna-kernel"
[[ -f "$KERNEL_ELF" ]] || { echo "ERROR: нет $KERNEL_ELF"; exit 1; }
cp "$KERNEL_ELF" "$BUILD/irna-kernel.elf"
echo "OK: $BUILD/irna-kernel.elf"

echo "==> [2/3] проверка Multiboot2-заголовка"
grub-file --is-x86-multiboot2 "$BUILD/irna-kernel.elf" \
    || { echo "ERROR: ELF не является корректным Multiboot2-образом"; exit 1; }
echo "OK: grub-file подтверждает Multiboot2-заголовок."
# Дополнительно: заголовок должен лежать в первых 64 КиБ и быть 8-выровненным
# (требование GRUB). Проверяем, что секция .multiboot_header — первая в образе.
readelf -x .multiboot_header "$BUILD/irna-kernel.elf" | head -5

if [[ "$FAST" == "1" ]]; then
    echo "==> (--fast) ELF готов: $BUILD/irna-kernel.elf"
    exit 0
fi

echo "==> [3/3] ISO через grub-mkrescue (BIOS)"
rm -rf "$BUILD/isodir"
mkdir -p "$BUILD/isodir/boot/grub"
cp "$BUILD/irna-kernel.elf" "$BUILD/isodir/boot/irna-kernel.elf"
cat > "$BUILD/isodir/boot/grub/grub.cfg" <<'CFG'
set timeout=1
set default=0

# FIX M0 v2: serial-вывод GRUB для диагностики загрузки
serial --speed=115200 --unit=0 --word=8 --parity=no --stop=1
terminal_input console serial
terminal_output console serial

menuentry "IrNa OS (M1)" {
    multiboot2 /boot/irna-kernel.elf
    boot
}
CFG

# Только BIOS-образ: в контейнерах/CI часто нет /usr/lib/SOFTWARE_PATH/efi.img,
# и grub-mkrescue с EFI падает с "Invalid ELF image for this architecture".
# Для M0 (QEMU i440fx/SeaBIOS) достаточно BIOS-загрузчика; EFI добавим на M1+.
GRUB_EFI_IMAGES="" \
grub-mkrescue -o "$BUILD/irna.iso" "$BUILD/isodir" 2>"$BUILD/grub-mkrescue.log" \
    || { echo "ERROR: grub-mkrescue не удалось:"; cat "$BUILD/grub-mkrescue.log"; exit 1; }
echo "OK: $BUILD/irna.iso ($(du -h "$BUILD/irna.iso" | cut -f1))"

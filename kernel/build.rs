// kernel/build.rs — сборка загрузочного trampoline (arch.asm) и линковка ядра.
// Этап M0. ИСПРАВЛЕНИЕ ДЕФЕКТА M0: раньше arch.asm компилировался отдельным
// `ld` вне cargo и не попадал в итоговый образ — ISO не загружался. Теперь:
//   1) nasm -f elf64 src/arch.asm -> build/arch.o (проверяем наличие nasm);
//   2) arch.o добавляется в линковку через `cargo:rustc-link-arg=<абс. путь>`;
//   3) линкер-скрипт linker.ld передаётся как `-T` (link-arg).
use std::process::Command;

fn main() {
    let out_dir = std::env::var("OUT_DIR").expect("OUT_DIR");
    let manifest = std::env::var("CARGO_MANIFEST_DIR").expect("CARGO_MANIFEST_DIR");
    let asm_src = format!("{manifest}/src/arch.asm");
    let obj = format!("{out_dir}/arch.o");
    let linker = format!("{manifest}/linker.ld");

    println!("cargo:rerun-if-changed=src/arch.asm");
    println!("cargo:rerun-if-changed=linker.ld");
    println!("cargo:rerun-if-changed=build.rs");

    // 1) Компиляция asm -> ELF64 объект
    let status = Command::new("nasm")
        .args(["-f", "elf64", &asm_src, "-o", &obj])
        .status()
        .expect("не найден nasm (apt install nasm)");
    assert!(status.success(), "nasm завершился с ошибкой для {asm_src}");

    // 2) Подключаем объект к линковке бинаря
    println!("cargo:rustc-link-arg={obj}");

    // 3) Собственный линкер-скрипт (Multiboot2-заголовок в начале образа, ENTRY=start)
    println!("cargo:rustc-link-arg=-T{linker}");
}

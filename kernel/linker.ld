# kernel/linker.ld — раскладка ELF-образа ядра ИрНа (M0)
# Загружается Multiboot2-загрузчиком GRUB; точка входа — start (boot.s).
ENTRY(start)

PHDRS
{
    headers PT_LOAD FLAGS(7); /* RWX: заголовок MB2 и код/данные в одном LOAD-сегменте (требование GRUB). M0 — сознанный компромисс. */
}

SECTIONS
{
    . = 1M;                     # ядро выше 1 МиБ (низ памяти — BIOS/вебра)

    .multiboot_header : {
        KEEP(*(.multiboot_header))
    } :headers

    .text : ALIGN(16) {
        *(.text .text.*)
    } :headers

    .rodata : {
        *(.rodata .rodata.*)
    } :headers

    .data : {
        *(.data .data.*)
        *(.got .got.*)
    } :headers

    .bss : {
        *(.bss .bss.*)
        *(COMMON)
    } :headers

    _end = .;                      # M1: метка конца образа ядра (pmm.rs помечает кадры ниже неё занятыми)

    /DISCARD/ : {
        *(.eh_frame)
        *(.comment)
        *(.note .note.*)
    }
}

; kernel/src/arch.asm — загрузочный trampoline ядра «ИрНа» (этап M0)
; Формат: ELF64 (nasm -f elf64), линкуется вместе с Rust-объектами.
; Схема (фиксируется как D-013): GRUB Multiboot2 передаёт управление в 32-битном
; protected mode на символ `start` (ENTRY в linker.ld). Здесь: PAE + LME,
; примитивные page tables (identity-map первых 16 МиБ huge-страницами), GDT,
; переход в long mode и вызов kernel_main (Rust).

MB2_MAGIC        equ 0xE85250D6
MB2_ARCH         equ 0
CR0_PE  equ 1 << 0
CR0_WP  equ 1 << 16
CR0_PG  equ 1 << 31
CR4_PAE equ 1 << 5
EFER_LME equ 1 << 8

; ИСПРАВЛЕНИЕ ДЕФЕКТА M1 (корневая причина):
; В заголовке НЕ было тега type=0 (information request). По спецификации
; Multiboot2 при ОТСУТСТВИИ такого запроса загрузчик обязан передать минимальный
; MBI без memory map (тег type=6) — что и наблюдалось: magic=1000 вместо
; 0x36D76289, free_mem_top=0. Теперь запрошен полный набор информации (flags=0);
; тег type=0 является header tag (не request), поэтому ограничение GRUB 2.06 на
; unsupported request-теги (0x14/framebuffer) его не затрагивает.
; ИСПРАВЛЕНИЕ ДЕФЕКТА M0 v4:
; GRUB 2.06 (Debian) не понимает request-теги framebuffer (unsupported tag 0x14) —
; их не добавляем. Точка входа = ELF entry (`start`).
; Serial-порт инициализируется ядром напрямую (COM1); текст рисуется в VGA RAM
; (0xB8000 identity-map'ится страницами ниже) — отдельного MB2-запроса не требуется.
; header_length = 16 (magic/arch/len/checksum) + 8 (terminator) = 24.
MB2_HEADER_LEN   equ 16 + 12 + 8       ; mandatory(16) + info-request(12) + end(8) = 36
MB2_CHECKSUM     equ (0 - (MB2_MAGIC + MB2_ARCH + MB2_HEADER_LEN)) & 0xFFFFFFFF ; mod 2^32

section .multiboot_header
align 8
mb2_start:
    dd MB2_MAGIC
    dd MB2_ARCH
    dd MB2_HEADER_LEN
    dd MB2_CHECKSUM

    ; Tag: information request (type=0) — запросить ВСЮ информацию загрузчика,
    ; включая memory map (tag 6). Без него GRUB передаёт урезанный MBI (дефект M1).
    dd 0                         ; type = information request
    dd 12                        ; size = 4+4+4 (type, size, flags)
    dd 0                         ; flags = 0 -> полный набор тегов

    ; End tag (type=0, size=8)
    dd 0                         ; type (end)
    dd 8                         ; size
mb2_end:

section .text
bits 32
extern kernel_main
global start
global mb_mboot_ptr           ; для Rust: указатель на структуру Multiboot2 (MBI)
global mb_mboot_ebx           ; диагностика M1: сырое значение EBX на входе (дубль)
start:
    cli
    ; EBX от GRUB = указатель на Multiboot-структуру. Сохраняем в памяти ядра
    ; (символ доступен из Rust) — M1 читает из неё memory map и загрузочные модули.
    mov [rel mb_mboot_ptr], ebx
    mov [rel mb_mboot_ebx], ebx   ; дубль для диагностики расхождения
    lea esp, [scratch_stack_end_32 - 4]

    ; CR4.PAE
    mov eax, cr4
    or eax, CR4_PAE
    mov cr4, eax

    ; EFER.LME (MSR 0xC0000080)
    mov ecx, 0xC0000080
    rdmsr
    or eax, EFER_LME
    wrmsr

    ; page tables (M1): pml4[0] -> pdpt; pdpt[0..1] -> pd_lo/pd_hi;
    ; huge-страницы: identity-map 0..512 МиБ (pd_lo) + mmio-область 512МиБ..1GiB
    ; (pd_hi, без флага RW — device memory). VGA 0xB8000 и PCI-устройства
    ; попадают в карту ниже 1 GiB.
    mov eax, pdpt
    or eax, 0x3
    mov [pml4], eax

    mov eax, pd_lo
    or eax, 0x3
    mov [pdpt], eax
    mov eax, pd_hi
    or eax, 0x3
    mov [pdpt + 8], eax

    ; pd_lo[i] = i*2MiB | P|RW|HUGE   (i = 0..255  =>  0..512 MiB)
    xor edi, edi
.build_lo:
    mov eax, edi
    shl eax, 21
    or eax, 0x83              ; PRESENT | WRITABLE | HUGE PAGE
    mov [pd_lo + edi*8], eax  ; старшие 32 бита записи = 0 (.bss обнулён)
    inc edi
    cmp edi, 256
    jb .build_lo

    ; pd_hi[j] = (512+j)*2MiB | P|HUGE (без WRITABLE: MMIO)  (j = 0..255 => до 1 GiB)
    xor edi, edi
.build_hi:
    mov eax, edi
    add eax, 256
    shl eax, 21
    or eax, 0x81              ; PRESENT | HUGE PAGE (read-only, device memory)
    mov [pd_hi + edi*8], eax
    inc edi
    cmp edi, 256
    jb .build_hi

    lea eax, [gdt64_ptr]             ; абсолютный адрес (адреса < 4 GiB на M0)
    lgdt [eax]                       ; КОСВЕННЫЙ доступ: в bits 32 линейный
                                     ; операнд `lgdt [gdt64_ptr]` читал бы GDTR-данные
                                     ; как указатель -> невалидный GDT base -> #GP/#PF
                                     ; при jmp в long mode (дефект M0).

    mov eax, pml4
    mov cr3, eax
    mov eax, cr0
    or eax, CR0_PG | CR0_WP | CR0_PE
    mov cr0, eax

    jmp dword 0x08:long_mode_entry

bits 64
long_mode_entry:
    mov ax, 0x10
    mov ds, ax
    mov es, ax
    mov ss, ax

    lea rsp, [rel stack_top]
    xor rbp, rbp
    ; M1: указатель на Multiboot-структуру сохранён в mb_mboot_ptr (см. start);
    ; Rust читает его напрямую — аргументы не передаём.
    call kernel_main
.halt:
    hlt
    jmp .halt

; --- M1: перезагрузка GDT+TSS из Rust (gdt.rs) -------------------------------
; void gdt_reload(u64 ptr /*rdi = адрес псевдо-дескриптора lgdt*/, u16 sel_tss /*esi*/)
; Делает: lgdt; ltr; far-jmp через retfq для загрузки CS; DS/ES/SS := data(0x10).
global gdt_reload
gdt_reload:
    lgdt [rdi]
    mov ax, si
    ltr ax
    push qword 0x08                ; новый CS-селектор
    lea rax, [rel gdt_reload.after]
    push rax
    lretq                          ; far return -> обновляет CS
gdt_reload.after:                  ; NASM: локальные метки через полное имя + `.local`
    mov ax, 0x10
    mov ds, ax
    mov es, ax
    mov ss, ax
    xor ax, ax                     ; TSS-регистр не требует чистки
    ret

section .bss
align 4096
pml4: resb 4096
pdpt: resb 4096
pd_lo: resb 4096
pd_hi: resb 4096
global mb_mboot_ptr
mb_mboot_ptr: resd 1          ; u32: указатель на Multiboot-структуру от GRUB (EBX)
mb_mboot_ebx: resd 1          ; u32: дубль EBX (диагностика M1)
align 16
stack_bottom: resb 16384      ; 16 KiB стек ядра (M0/M1)
stack_top:
align 16
global scratch_stack_bottom_32
scratch_stack_bottom_32: resb 256   ; маленький стек в .bss (identity-map до 16MiB)
scratch_stack_end_32:
align 16
; M1: экспорт границ основного стека ядра для TSS.RSP0 (gdt.rs)
global stack_bottom_addr
stack_bottom_addr: equ stack_bottom
global stack_top_addr
stack_top_addr: equ stack_top
; M1: IST1-стек для #DF
align 4096
global df_stack_bottom_addr
df_stack_bottom_addr: resb 4096
global df_stack_top_addr
df_stack_top_addr:


section .rodata
align 8
gdt64:
    dq 0                                  ; 0x00 null
    dq (1<<41) | (1<<43) | (1<<44) | (1<<47) | (1<<53) ; 0x08 code64: RW|X|L|P|D(64-bit)
    dq (1<<41) | (1<<44) | (1<<47)        ; 0x10 data: RW|P|D
gdt64_end:
; Ограничения/базы сегментов в long mode игнорируются процессором (кроме CS base,
; который тоже берётся из GDT при far jmp — но для данных base должен быть корректным
; адресом таблицы: lgdt грузит базу из этого псевдо-дескриптора целиком.
; Раньше base был 0 -> любой доступ к сегментам данных -> #GP -> triple fault
; (дефект M0: загрузка зависала до kernel_main, serial-лог пустой).
gdt64_ptr:
    dw gdt64_end - gdt64 - 1              ; limit = размер таблицы - 1
    dd gdt64                              ; base: младшие 32 бита (адрес < 4 GiB)
    dd 0                                  ; base: старшие 32 бита

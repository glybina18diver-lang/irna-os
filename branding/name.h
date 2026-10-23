// branding/name.h — константы имени ОС «ИрНа» (реестр: branding.md, строки B1)
// Правило D-008: имя ОС НЕ хардкодится нигде, кроме этого файла и его аналогов
// (kernel/src/brand.rs — B3, bootloader/config.toml — B2).
#ifndef IRNA_BRANDING_NAME_H
#define IRNA_BRANDING_NAME_H

// Отображаемое имя (UTF-8, кириллица). Правка этой строки = переименование UI. (B1)
#define BRAND_NAME_DISPLAY "ИрНа"

// Технический идентификатор (ASCII, нижний регистр). Не меняется при ребрендинге UI.
#define BRAND_ID "irna"

#endif // IRNA_BRANDING_NAME_H

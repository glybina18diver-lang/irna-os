# tools/gen-font8x16.ps1 — генерация kernel/src/font.rs из PT Mono (OFL).
#
# Почему PT Mono: D-011 фиксирует PT Sans для интерфейса и PT Mono для
# терминала/кода. Консоль ядра — это терминал, поэтому битмап 8x16 строится
# из PT Mono. Шрифт масштабируемый, поэтому глифы растрируются в сетку 10x16
# (моноширинный, без сглаживания) и упаковываются в таблицу для framebuffer.
#
# Использование:
#   powershell -NoProfile -ExecutionPolicy Bypass -File tools\gen-font8x16.ps1 `
#     -Ttf C:\path\PTM55FT.ttf -Out kernel\src\font8x16.rs `
#     [-Size 12] [-Baseline 12] [-XOffset 0] [-CellW 10]
#
# Превью шрифта: tools/preview-font8x16.ps1 (читает готовый .rs).

param(
    [Parameter(Mandatory = $true)][string]$Ttf,
    [string]$Out = "$PSScriptRoot\..\kernel\src\font.rs",
    [string]$Preview = "",
    [int]$Size = 12,
    [int]$Baseline = 12,
    [int]$XOffset = 0,
    [int]$CellW = 10
)

$ErrorActionPreference = "Stop"
Add-Type -AssemblyName System.Drawing

$pfc = New-Object System.Drawing.Text.PrivateFontCollection
$pfc.AddFontFile((Resolve-Path $Ttf).Path)
$family = $pfc.Families[0]
$font = New-Object System.Drawing.Font($family, [single]$Size, [System.Drawing.FontStyle]::Regular, [System.Drawing.GraphicsUnit]::Pixel)

# Набор: ASCII 0x20..0x7E, кириллица 0x0410..0x044F, Ё/ё.
$cps = New-Object System.Collections.Generic.List[int]
for ($c = 0x20; $c -le 0x7E; $c++) { $cps.Add($c) }
for ($c = 0x0410; $c -le 0x044F; $c++) { $cps.Add($c) }
$cps.Add(0x0401)
$cps.Add(0x0451)
$cps.Add(0x2013)  # –
$cps.Add(0x2014)  # —
$cps.Add(0x00AB)  # «
$cps.Add(0x00BB)  # »

$asciiCount = 0x7E - 0x20 + 1
$cyrCount = 0x044F - 0x0410 + 1
$total = $asciiCount + $cyrCount + 6

$sf = [System.Drawing.StringFormat]::GenericTypographic
$sf.FormatFlags = $sf.FormatFlags -bor [System.Drawing.StringFormatFlags]::NoClip

function Get-Glyph([int]$cp) {
    $bmp = New-Object System.Drawing.Bitmap $CellW, 16
    $g = [System.Drawing.Graphics]::FromImage($bmp)
    $g.Clear([System.Drawing.Color]::Black)
    $g.TextRenderingHint = [System.Drawing.Text.TextRenderingHint]::SingleBitPerPixelGridFit
    $s = [string][char]$cp
    $w = $g.MeasureString($s, $font, 1000, $sf).Width
    $x = [math]::Floor(($CellW - $w) / 2) + $XOffset
    $brush = [System.Drawing.Brushes]::White
    $g.DrawString($s, $font, $brush, [single]$x, [single]($Baseline - $Size), $sf)
    $g.Dispose()
    $rows = New-Object System.Collections.Generic.List[string]
    for ($y = 0; $y -lt 16; $y++) {
        $word = 0
        for ($x2 = 0; $x2 -lt $CellW; $x2++) {
            $c = $bmp.GetPixel($x2, $y)
            if (($c.R + $c.G + $c.B) -gt 300) { $word = $word -bor (0x8000 -shr $x2) }
        }
        $rows.Add(("0x{0:X4}" -f $word))
    }
    $bmp.Dispose()
    return ,$rows
}

$all = @{}
foreach ($cp in $cps) { $all[$cp] = Get-Glyph $cp }

$sb = New-Object System.Text.StringBuilder
[void]$sb.AppendLine("// kernel/src/font.rs — растровый шрифт консоли (ASCII + кириллица).")
[void]$sb.AppendLine("//")
[void]$sb.AppendLine("// СГЕНЕРИРОВАНО автоматически: tools/gen-font8x16.ps1")
[void]$sb.AppendLine("// Источник: PT Mono Regular (PTM55FT.ttf), ParaType, SIL Open Font License 1.1.")
[void]$sb.AppendLine("// Лицензия: см. assets/fonts/OFL-PT-Mono.txt. Шрифт растрирован в сетку 8x16")
[void]$sb.AppendLine("// без сглаживания (SingleBitPerPixelGridFit), по одному u16 на строку (10 пикселей),")
[void]$sb.AppendLine("// MSB — левый пиксель. Это терминальный шрифт консоли (D-011: PT Mono).")
[void]$sb.AppendLine("//")
[void]$sb.AppendLine("// Не редактировать вручную: перегенерировать скриптом.")
[void]$sb.AppendLine()
[void]$sb.AppendLine("pub const GLYPH_W: usize = $CellW;")
[void]$sb.AppendLine("pub const GLYPH_H: usize = 16;")
[void]$sb.AppendLine()
[void]$sb.AppendLine("const ASCII_FIRST: u32 = 0x20;")
[void]$sb.AppendLine("const ASCII_LAST: u32 = 0x7E;")
[void]$sb.AppendLine("const CYR_FIRST: u32 = 0x0410;")
[void]$sb.AppendLine("const CYR_LAST: u32 = 0x044F;")
[void]$sb.AppendLine("const ASCII_COUNT: usize = 95;")
[void]$sb.AppendLine()
[void]$sb.AppendLine("/// Индекс глифа в `GLYPHS`. Неизвестный символ отображается как '?'.")
[void]$sb.AppendLine("pub fn glyph_index(c: char) -> usize {")
[void]$sb.AppendLine("    let u = c as u32;")
[void]$sb.AppendLine("    if (ASCII_FIRST..=ASCII_LAST).contains(&u) {")
[void]$sb.AppendLine("        return (u - ASCII_FIRST) as usize;")
[void]$sb.AppendLine("    }")
[void]$sb.AppendLine("    if (CYR_FIRST..=CYR_LAST).contains(&u) {")
[void]$sb.AppendLine("        return ASCII_COUNT + (u - CYR_FIRST) as usize;")
[void]$sb.AppendLine("    }")
[void]$sb.AppendLine("    match u {")
[void]$sb.AppendLine("        0x0401 => ASCII_COUNT + 64, // Ё")
[void]$sb.AppendLine("        0x0451 => ASCII_COUNT + 65, // ё")
[void]$sb.AppendLine("        0x2013 => ASCII_COUNT + 66, // –")
[void]$sb.AppendLine("        0x2014 => ASCII_COUNT + 67, // —")
[void]$sb.AppendLine("        0x00AB => ASCII_COUNT + 68, // «")
[void]$sb.AppendLine("        0x00BB => ASCII_COUNT + 69, // »")
[void]$sb.AppendLine("        _ => ('?' as u32 - ASCII_FIRST) as usize,")
[void]$sb.AppendLine("    }")
[void]$sb.AppendLine("}")
[void]$sb.AppendLine()
[void]$sb.AppendLine("/// $total глифов: 95 ASCII + 64 кириллических + Ё/ё + типографика.")
[void]$sb.AppendLine("pub static GLYPHS: [[u16; GLYPH_H]; $total] = [")
$idx = 0
foreach ($cp in $cps) {
    [void]$sb.AppendLine(("    [" + ($all[$cp] -join ", ") + "], // U+{0:X4}") -f $cp)
    $idx++
}
[void]$sb.AppendLine("];")
[System.IO.File]::WriteAllText($Out, $sb.ToString(), (New-Object System.Text.UTF8Encoding($false)))
Write-Host "OK: $Out ($idx глифов)"

# tools/preview-font8x16.ps1 — превью сгенерированного шрифта (для проверки глазами).
# Читает kernel/src/font8x16.rs и рисует все глифы сеткой; вспомогательный
# инструмент разработки, в ядро не входит.
param(
    [string]$Rs = "$PSScriptRoot\..\kernel\src\font8x16.rs",
    [string]$Out = "$PSScriptRoot\..\artifacts\font-preview.png",
    [int]$Scale = 4
)
$ErrorActionPreference = "Stop"
Add-Type -AssemblyName System.Drawing

$text = [System.IO.File]::ReadAllText($Rs)
$cw = [int]([regex]::Match($text, 'GLYPH_W: usize = (\d+)').Groups[1].Value)
$rows = [regex]::Matches($text, '\[((?:0x[0-9A-F]{4},\s*){15}0x[0-9A-F]{4})\]')
$n = $rows.Count
$cols = 16
$rowsN = [math]::Ceiling($n / $cols)
$cellW = ($cw + 2) * $Scale
$cellH = 18 * $Scale
$bmp = New-Object System.Drawing.Bitmap ($cols * $cellW), ($rowsN * $cellH)
$g = [System.Drawing.Graphics]::FromImage($bmp)
$g.Clear([System.Drawing.Color]::Black)
for ($i = 0; $i -lt $n; $i++) {
    $vals = $rows[$i].Groups[1].Value.Split(',') | ForEach-Object { [Convert]::ToInt32($_.Trim(), 16) }
    $gx = ($i % $cols) * $cellW
    $gy = [math]::Floor($i / $cols) * $cellH
    for ($y = 0; $y -lt 16; $y++) {
        for ($x = 0; $x -lt $cw; $x++) {
            if ($vals[$y] -band (0x8000 -shr $x)) {
                $g.FillRectangle([System.Drawing.Brushes]::White, ($gx + $x * $Scale), ($gy + $y * $Scale), $Scale, $Scale)
            }
        }
    }
}
$g.Dispose()
New-Item -ItemType Directory -Force (Split-Path $Out) | Out-Null
$bmp.Save($Out, [System.Drawing.Imaging.ImageFormat]::Png)
$bmp.Dispose()
Write-Host "OK: $Out ($n глифов, ячейка ${cw}x16)"

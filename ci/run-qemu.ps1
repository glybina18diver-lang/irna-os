# ci/run-qemu.ps1 — локальная проверка ISO «ИрНа» в QEMU под Windows.
#
# Зачем отдельный скрипт: на ноутбуке нет WSL, а `ci/run-qemu.sh` требует bash.
# Serial выводится через TCP (без буферизации, в отличие от `-serial file:`),
# Enter в меню GRUB отправляется символом CR по serial.
#
# Использование:
#   powershell -NoProfile -ExecutionPolicy Bypass -File ci\run-qemu.ps1
#   powershell ... -File ci\run-qemu.ps1 -Iso C:\path\irna.iso -Seconds 30
#
# Код возврата 0 — все ожидаемые маркеры найдены.

param(
    [string]$Iso = "$PSScriptRoot\..\artifacts\irna.iso",
    [int]$SerialPort = 4600,
    [int]$Seconds = 30,
    [int]$EnterAt = 6,
    [string]$Log = "$PSScriptRoot\..\artifacts\logs\qemu-serial.log"
)

$ErrorActionPreference = "Stop"

$qemu = "C:\Program Files\qemu\qemu-system-x86_64.exe"
if (-not (Test-Path $qemu)) { $qemu = "C:\Projects\OS\tools\qemu\qemu-system-x86_64.exe" }
if (-not (Test-Path $Iso)) { Write-Error "нет ISO: $Iso"; exit 1 }

New-Item -ItemType Directory -Force (Split-Path $Log) | Out-Null
Remove-Item $Log -Force -ErrorAction SilentlyContinue

Write-Host "==> QEMU (BIOS, headless, serial TCP): $Iso ($Seconds c)"

$p = Start-Process -FilePath $qemu -ArgumentList @(
    "-cdrom", (Resolve-Path $Iso).Path,
    "-m", "512",
    "-display", "none",
    "-no-reboot",
    "-serial", "tcp:127.0.0.1:$SerialPort,server=on,wait=on"
) -PassThru
Start-Sleep -Seconds 1

$client = New-Object System.Net.Sockets.TcpClient
$connected = $false
for ($i = 0; $i -lt 40; $i++) {
    try { $client.Connect("127.0.0.1", $SerialPort); $connected = $true; break }
    catch { Start-Sleep -Milliseconds 250 }
}
if (-not $connected) {
    if (-not $p.HasExited) { $p.Kill() }
    Write-Error "не удалось подключиться к serial-порту QEMU"
    exit 1
}

# Собираем сырые байты: serial — UTF-8, декодируем один раз в конце,
# чтобы не разрезать многобайтные символы (кириллицу) между чтениями.
$stream = $client.GetStream()
$stream.ReadTimeout = 200
$bytes = New-Object System.Collections.Generic.List[byte]
$buf = New-Object byte[] 8192
$start = Get-Date
$sent = $false

while (((Get-Date) - $start).TotalSeconds -lt $Seconds) {
    if (-not $sent -and ((Get-Date) - $start).TotalSeconds -ge $EnterAt) {
        $cr = [byte[]](0x0D)
        $stream.Write($cr, 0, 1); $stream.Flush()
        $sent = $true
    }
    try {
        $n = $stream.Read($buf, 0, $buf.Length)
        if ($n -gt 0) { $bytes.AddRange([byte[]]$buf[0..($n - 1)]) }
    } catch [System.IO.IOException] { }
}
$client.Close()
$exited = $p.HasExited
if (-not $exited) { $p.Kill() }

$out = [System.Text.Encoding]::UTF8.GetString($bytes.ToArray())
Set-Content -Path $Log -Value $out -Encoding UTF8
# Читаемая копия без ANSI-последовательностей GRUB.
$clean = $out -replace "\x1b\[[0-9;?]*[A-Za-z]", ""
Set-Content -Path ($Log -replace "\.log$", ".clean.log") -Value $clean -Encoding UTF8

$checks = @(
    @{ Name = "загрузка ядра";       Pattern = "M1 subsystems OK" },
    @{ Name = "MBI разобран";        Pattern = "MBI found\s*=\s*1" },
    @{ Name = "IDT загружена";       Pattern = "IDT loaded" },
    @{ Name = "smoke-тест int3";     Pattern = "IDT int3 smoke OK" }
)

$fail = 0
foreach ($c in $checks) {
    if ($clean -match $c.Pattern) {
        Write-Host ("    OK: " + $c.Name)
    } else {
        Write-Host ("    FAIL: " + $c.Name)
        $fail++
    }
}

Write-Host "==> лог: $Log"
if ($fail -ne 0) { exit 1 }
Write-Host "==> SUCCESS: ISO загрузился, IDT работает."
exit 0

# ci/run-qemu.ps1 — локальная проверка ISO «ИрНа» в QEMU под Windows.
#
# Зачем отдельный скрипт: на ноутбуке нет WSL, а `ci/run-qemu.sh` требует bash.
# Serial выводится через TCP (без буферизации, в отличие от `-serial file:`),
# Enter в меню GRUB отправляется символом CR по serial.
#
# Клавиша для проверки IRQ1 отправляется через QMP `input-send-event`.
# HMP-команда `sendkey` в headless-режиме (`-display none`) не доходит до
# PS/2-клавиатуры, а QMP с явным событием — доходит.
#
# Использование:
#   powershell -NoProfile -ExecutionPolicy Bypass -File ci\run-qemu.ps1
#   powershell ... -File ci\run-qemu.ps1 -Iso C:\path\irna.iso -Seconds 30
#   powershell ... -File ci\run-qemu.ps1 -Key ""   # без проверки клавиатуры
#
# Код возврата 0 — все ожидаемые маркеры найдены.

param(
    [string]$Iso = "$PSScriptRoot\..\artifacts\irna.iso",
    [int]$SerialPort = 4600,
    [int]$Seconds = 30,
    [int]$EnterAt = 6,
    [int]$QmpPort = 4601,
    [string]$Key = "a",
    [int]$KeyAt = 6,
    [string]$Log = "$PSScriptRoot\..\artifacts\logs\qemu-serial.log",
    [string]$Screenshot = "",
    [int]$ScreenshotAt = 0
)

$ErrorActionPreference = "Stop"
[Console]::OutputEncoding = [System.Text.Encoding]::UTF8

$qemu = "C:\Program Files\qemu\qemu-system-x86_64.exe"
if (-not (Test-Path $qemu)) { $qemu = "C:\Projects\OS\tools\qemu\qemu-system-x86_64.exe" }
if (-not (Test-Path $Iso)) { Write-Error "нет ISO: $Iso"; exit 1 }

New-Item -ItemType Directory -Force (Split-Path $Log) | Out-Null
Remove-Item $Log -Force -ErrorAction SilentlyContinue
$errLog = $Log -replace "\.log$", ".qemu-err.log"

Write-Host "==> QEMU (BIOS, headless, serial TCP): $Iso ($Seconds c)"

# -WindowStyle Hidden + RedirectStandardError: QEMU не должен открывать
# отдельное консольное окно на рабочем столе.
$p = Start-Process -FilePath $qemu -ArgumentList @(
    "-cdrom", (Resolve-Path $Iso).Path,
    "-m", "512",
    "-display", "none",
    "-no-reboot",
    "-serial", "tcp:127.0.0.1:$SerialPort,server=on,wait=on",
    "-qmp", "tcp:127.0.0.1:$QmpPort,server=on,wait=off"
) -PassThru -WindowStyle Hidden -RedirectStandardError $errLog
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

$stream = $client.GetStream()
$stream.ReadTimeout = 200
$bytes = New-Object System.Collections.Generic.List[byte]
$buf = New-Object byte[] 8192
$start = Get-Date
$sent = $false

# --- QMP: отправка клавиши (проверка IRQ1) ---
$qmp = $null
$qstream = $null
$qbuf = New-Object byte[] 65536

function Send-Qmp($obj) {
    $json = $obj | ConvertTo-Json -Compress -Depth 8
    $b = [Text.Encoding]::ASCII.GetBytes($json + "`n")
    $qstream.Write($b, 0, $b.Length)
    $qstream.Flush()
    Start-Sleep -Milliseconds 150
    try { $null = $qstream.Read($qbuf, 0, $qbuf.Length) } catch { }
}

if ($Key -ne "" -or $Screenshot -ne "") {
    $qmp = New-Object System.Net.Sockets.TcpClient
    for ($i = 0; $i -lt 20; $i++) {
        try { $qmp.Connect("127.0.0.1", $QmpPort); break } catch { Start-Sleep -Milliseconds 250 }
    }
    if ($qmp.Connected) {
        $qstream = $qmp.GetStream()
        $qstream.ReadTimeout = 500
        try { $null = $qstream.Read($qbuf, 0, $qbuf.Length) } catch { }  # приветствие QMP
        Send-Qmp @{ execute = "qmp_capabilities" }
    } else {
        $qmp = $null
        Write-Host "    WARN: QMP недоступен, клавиша не будет нажата"
    }
}
$keySent = $false
$shotSent = $false
$keySentAt = 0.0
$acc = New-Object System.Text.StringBuilder
if ($Screenshot -ne "" -and $ScreenshotAt -le 0) { $ScreenshotAt = 0 }

while (((Get-Date) - $start).TotalSeconds -lt $Seconds) {
    try {
        $n = $stream.Read($buf, 0, $buf.Length)
        if ($n -gt 0) {
            $bytes.AddRange([byte[]]$buf[0..($n - 1)])
            [void]$acc.Append([System.Text.Encoding]::ASCII.GetString($buf, 0, $n))
        }
    } catch [System.IO.IOException] { }

    $elapsed = ((Get-Date) - $start).TotalSeconds

    if (-not $sent -and $elapsed -ge $EnterAt) {
        $cr = [byte[]](0x0D)
        $stream.Write($cr, 0, 1); $stream.Flush()
        $sent = $true
    }

    # Клавиша отправляется не по абсолютному таймеру, а когда ядро уже в цикле
    # эха ("keyboard echo:"): на слабом CPU загрузка с видеорежимом занимает
    # заметно больше времени, и жёсткий KeyAt промахивался.
    if ($qmp -and $Key -ne "" -and -not $keySent -and $elapsed -ge $KeyAt -and $acc.ToString().Contains("keyboard echo:")) {
        $events = @(
            @{ type = "key"; data = @{ down = $true; key = @{ type = "qcode"; data = $Key } } },
            @{ type = "key"; data = @{ down = $false; key = @{ type = "qcode"; data = $Key } } }
        )
        Send-Qmp @{ execute = "input-send-event"; arguments = @{ events = $events } }
        $keySent = $true
        $keySentAt = $elapsed
    }

    if ($qmp -and $Screenshot -ne "" -and -not $shotSent) {
        $ready = if ($Key -ne "") { $keySent -and $elapsed -ge ($keySentAt + 1) }
                 else { $ScreenshotAt -gt 0 -and $elapsed -ge $ScreenshotAt }
        if ($ready) {
            Send-Qmp @{ execute = "screendump"; arguments = @{ filename = $Screenshot; format = "png" } }
            $shotSent = $true
        }
    }
}
$client.Close()
if ($qmp) { $qmp.Close() }
$exited = $p.HasExited
if (-not $exited) { $p.Kill() }

# Serial — UTF-8; декодируем один раз, чтобы не разрезать многобайтные символы.
$out = [System.Text.Encoding]::UTF8.GetString($bytes.ToArray())
Set-Content -Path $Log -Value $out -Encoding UTF8
$clean = $out -replace "\x1b\[[0-9;?]*[A-Za-z]", ""
Set-Content -Path ($Log -replace "\.log$", ".clean.log") -Value $clean -Encoding UTF8

$checks = @(
    @{ Name = "загрузка ядра";       Pattern = "M1 subsystems OK" },
    @{ Name = "MBI разобран";        Pattern = "MBI found\s*=\s*1" },
    @{ Name = "IDT загружена";       Pattern = "IDT loaded" },
    @{ Name = "smoke-тест int3";     Pattern = "IDT int3 smoke OK" },
    @{ Name = "PIC/PIT: IRQ0";       Pattern = "PIT: 20 ticks OK" },
    @{ Name = "PS/2 клавиатура";     Pattern = "PS/2 keyboard ready" },
    @{ Name = "framebuffer (LFB)";   Pattern = "framebuffer console OK" },
    @{ Name = "SYSCALL/SYSRET";      Pattern = "SYSCALL/SYSRET enabled" },
    @{ Name = "ring 3 round-trip";   Pattern = "syscall/sysret round-trip OK" },
    @{ Name = "планировщик";         Pattern = "sched: tasks done\s*=\s*3" }
)

if ($Key -ne "") {
    $checks += @{ Name = "клавиша '$Key' по IRQ1"; Pattern = "key: '$Key'" }
}

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
Write-Host "==> SUCCESS: ISO загрузился, прерывания и клавиатура работают."
exit 0

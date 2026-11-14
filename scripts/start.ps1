$ErrorActionPreference = 'Stop'
$projectRoot = Split-Path -Parent $PSScriptRoot
Set-Location -LiteralPath $projectRoot
$port = if ($env:CASSETTE_PORT) { $env:CASSETTE_PORT } else { '3210' }
$alreadyRunning = $false
try {
    $existing = Invoke-RestMethod "http://127.0.0.1:$port/api/status" -TimeoutSec 2
    $alreadyRunning = [bool]$existing.version
} catch { }
if ($alreadyRunning) {
    Start-Process "http://127.0.0.1:$port/"
    Write-Host 'Cassette Studio is already running.'
    return
}
$exe = Join-Path $projectRoot 'target\release\cassette-studio.exe'
if (-not (Test-Path -LiteralPath $exe)) {
    & cargo build --release --locked
    if ($LASTEXITCODE -ne 0) { throw 'Build failed.' }
}
$logBase = Join-Path ([System.IO.Path]::GetTempPath()) ("cassette-studio-" + [guid]::NewGuid().ToString('N'))
$outputLog = "$logBase.stdout.log"
$errorLog = "$logBase.stderr.log"
$server = Start-Process -FilePath $exe -WorkingDirectory $projectRoot -WindowStyle Hidden -PassThru -RedirectStandardOutput $outputLog -RedirectStandardError $errorLog
try {
    $ready = $false
    $serverUrl = $null
    for ($attempt = 0; $attempt -lt 60; $attempt++) {
        if ($server.HasExited) {
            $details = if (Test-Path -LiteralPath $errorLog) { Get-Content -LiteralPath $errorLog -Raw } else { '' }
            throw "The server exited. Close another Cassette Studio instance using this library, or check the configured port. $details"
        }
        if (Test-Path -LiteralPath $outputLog) {
            $text = Get-Content -LiteralPath $outputLog -Raw
            if ($text -match 'http://127\.0\.0\.1:\d+') { $serverUrl = $Matches[0] }
        }
        if ($serverUrl) {
            try { Invoke-RestMethod "$serverUrl/api/library" -TimeoutSec 2 | Out-Null; $ready = $true; break } catch { }
        }
        Start-Sleep -Milliseconds 500
    }
    if (-not $ready) { throw 'Startup timed out.' }
    Start-Process "$serverUrl/"
    Write-Host "Cassette Studio: $serverUrl/"
    Write-Host 'Press Enter to stop Cassette Studio.'
    Read-Host | Out-Null
} finally {
    if (-not $server.HasExited) { Stop-Process -Id $server.Id }
    foreach ($path in @($outputLog, $errorLog)) { if (Test-Path -LiteralPath $path) { Remove-Item -LiteralPath $path -ErrorAction SilentlyContinue } }
}

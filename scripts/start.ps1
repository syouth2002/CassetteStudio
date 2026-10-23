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
$server = Start-Process -FilePath $exe -WorkingDirectory $projectRoot -WindowStyle Hidden -PassThru
try {
    $ready = $false
    for ($attempt = 0; $attempt -lt 60; $attempt++) {
        if ($server.HasExited) { throw 'The server exited. Port 3210 may already be in use.' }
        try { Invoke-RestMethod "http://127.0.0.1:$port/api/library" | Out-Null; $ready = $true; break } catch { Start-Sleep -Milliseconds 500 }
    }
    if (-not $ready) { throw 'Startup timed out.' }
    Start-Process "http://127.0.0.1:$port/"
    Write-Host "Cassette Studio: http://127.0.0.1:$port/"
    Write-Host 'Press Enter to stop Cassette Studio.'
    Read-Host | Out-Null
} finally { if (-not $server.HasExited) { Stop-Process -Id $server.Id } }

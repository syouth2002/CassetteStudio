param([string]$Version = '0.1.0')
$ErrorActionPreference = 'Stop'
$root = Split-Path -Parent $PSScriptRoot
Push-Location -LiteralPath $root
try {
    & cargo build --release --features desktop --bins --locked
    if ($LASTEXITCODE -ne 0) { throw 'Build failed.' }
    $stage = Join-Path $root "dist\releases\v$Version\windows-x64"
    if (Test-Path -LiteralPath $stage) { throw "Stage already exists: $stage. Use a new clean staging path." }
    New-Item -ItemType Directory -Force $stage, "$stage\scripts", "$stage\docs" | Out-Null
    Copy-Item target\release\cassette-studio-desktop.exe "$stage\CassetteStudio.exe"
    Copy-Item target\release\cassette-studio.exe "$stage\cassette-studio-browser.exe"
    Copy-Item scripts\setup-tools.ps1 "$stage\scripts\setup-tools.ps1"
    '@echo off', 'powershell -NoProfile -ExecutionPolicy Bypass -File "%~dp0scripts\setup-tools.ps1"', 'pause' | Set-Content "$stage\SetupAudioTools.cmd" -Encoding ascii
    'Portable library: data/ beside the application.' | Set-Content "$stage\portable.txt" -Encoding ascii
    foreach ($file in @('README.md','README.ja.md','LICENSE','THIRD_PARTY_NOTICES.md')) { Copy-Item -LiteralPath $file -Destination $stage }
    Copy-Item docs\screenshots "$stage\docs\screenshots" -Recurse
    & python scripts\collect-licenses.py --target x86_64-pc-windows-msvc --output "$stage\licenses"
    if ($LASTEXITCODE -ne 0) { throw 'License collection failed.' }
    & python scripts\archive-release.py "$stage" "dist\releases\v$Version\CassetteStudio-$Version-Windows-x64.zip"
    if ($LASTEXITCODE -ne 0) { throw 'Archive creation failed.' }
} finally { Pop-Location }

$ErrorActionPreference = 'Stop'
$projectRoot = Split-Path -Parent $PSScriptRoot
$exe = Join-Path $projectRoot 'CassetteStudio.exe'
if (-not (Test-Path -LiteralPath $exe)) {
    Push-Location -LiteralPath $projectRoot
    try {
        & cargo build --release --features desktop --bin cassette-studio-desktop --locked
        if ($LASTEXITCODE -ne 0) { throw 'Desktop build failed.' }
        Copy-Item -LiteralPath (Join-Path $projectRoot 'target\release\cassette-studio-desktop.exe') -Destination $exe
    } finally { Pop-Location }
}
Start-Process -FilePath $exe -WorkingDirectory $projectRoot

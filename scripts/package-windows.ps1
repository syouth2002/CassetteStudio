$ErrorActionPreference = 'Stop'
$projectRoot = Split-Path -Parent $PSScriptRoot
Push-Location -LiteralPath $projectRoot
try {
    & cargo build --release --features desktop --bins --locked
    if ($LASTEXITCODE -ne 0) { throw 'Build failed.' }
    $destination = Join-Path $projectRoot 'dist\windows-x64'
    $toolsTarget = Join-Path $destination 'tools'
    New-Item -ItemType Directory -Force $destination, $toolsTarget | Out-Null
    New-Item -ItemType Directory -Force "$destination\assets" | Out-Null
    Copy-Item assets\cassette-studio-app.png "$destination\assets\cassette-studio-app.png" -Force
    Copy-Item -LiteralPath 'target\release\cassette-studio-desktop.exe' -Destination (Join-Path $destination 'CassetteStudio.exe') -Force
    Copy-Item -LiteralPath 'target\release\cassette-studio.exe' -Destination (Join-Path $destination 'cassette-studio-browser.exe') -Force
    foreach ($file in @('ffmpeg.exe','ffprobe.exe','yt-dlp.exe','FFmpeg-LICENSE.txt')) {
        $source = Join-Path $projectRoot "tools\$file"
        if (-not (Test-Path -LiteralPath $source)) { throw "Run setup-tools.ps1 first: missing $file" }
        Copy-Item -LiteralPath $source -Destination (Join-Path $toolsTarget $file) -Force
    }
    $nodeBinary = (Get-Command node.exe -ErrorAction Stop).Source
    Copy-Item -LiteralPath $nodeBinary -Destination (Join-Path $toolsTarget 'node.exe') -Force
    foreach ($document in @('README.md','README.ja.md','LICENSE','THIRD_PARTY_NOTICES.md')) {
        Copy-Item -LiteralPath $document -Destination (Join-Path $destination $document) -Force
    }
    Copy-Item -LiteralPath 'docs' -Destination (Join-Path $destination 'docs') -Recurse -Force
    'Keep this file to save this portable copy''s library in data/ beside the application.' | Set-Content -LiteralPath (Join-Path $destination 'portable.txt') -Encoding utf8
    # Runtime dependencies are downloaded from official distribution sites.
    Invoke-WebRequest -UseBasicParsing 'https://raw.githubusercontent.com/yt-dlp/yt-dlp/master/LICENSE' -OutFile (Join-Path $toolsTarget 'yt-dlp-LICENSE.txt')
    $nodeVersion = (& node.exe --version).Trim()
    Invoke-WebRequest -UseBasicParsing "https://raw.githubusercontent.com/nodejs/node/$nodeVersion/LICENSE" -OutFile (Join-Path $toolsTarget 'Node-LICENSE.txt')
    Compress-Archive -LiteralPath $destination -DestinationPath (Join-Path $projectRoot 'dist\CassetteStudio-Windows-x64.zip') -Force
    Copy-Item -LiteralPath 'target\release\cassette-studio-desktop.exe' -Destination (Join-Path $projectRoot 'CassetteStudio.exe') -Force
    Write-Host "Ready: $destination"
} finally { Pop-Location }

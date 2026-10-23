$ErrorActionPreference = 'Stop'
$projectRoot = Split-Path -Parent $PSScriptRoot
$toolsDir = Join-Path $projectRoot 'tools'
$downloadDir = Join-Path $projectRoot 'work\tool-download'
New-Item -ItemType Directory -Force $toolsDir, $downloadDir | Out-Null
$ProgressPreference = 'SilentlyContinue'
if (-not (Test-Path -LiteralPath (Join-Path $toolsDir 'ffmpeg.exe'))) {
    Write-Host 'Downloading FFmpeg (Windows build linked from ffmpeg.org)...'
    $archive = Join-Path $downloadDir 'ffmpeg.zip'
    if (-not (Test-Path -LiteralPath $archive)) { Invoke-WebRequest -UseBasicParsing 'https://www.gyan.dev/ffmpeg/builds/ffmpeg-release-essentials.zip' -OutFile $archive }
    $checksum = (Invoke-WebRequest -UseBasicParsing 'https://www.gyan.dev/ffmpeg/builds/ffmpeg-release-essentials.zip.sha256').Content
    $expected = ([regex]::Match([string]$checksum, '[a-fA-F0-9]{64}')).Value
    if (-not $expected -or (Get-FileHash -LiteralPath $archive -Algorithm SHA256).Hash -ne $expected) { throw 'FFmpeg checksum mismatch.' }
    Expand-Archive -LiteralPath $archive -DestinationPath $downloadDir -Force
    foreach ($name in @('ffmpeg.exe', 'ffprobe.exe')) {
        $binary = Get-ChildItem -LiteralPath $downloadDir -Recurse -Filter $name | Select-Object -First 1
        if (-not $binary) { throw "Missing binary: $name" }
        Copy-Item -LiteralPath $binary.FullName -Destination (Join-Path $toolsDir $name)
    }
    $license = Get-ChildItem -LiteralPath $downloadDir -Recurse -Filter LICENSE | Select-Object -First 1
    if ($license) { Copy-Item -LiteralPath $license.FullName -Destination (Join-Path $toolsDir 'FFmpeg-LICENSE.txt') }
}
Write-Host 'Downloading yt-dlp from its official GitHub release...'
$release = Invoke-RestMethod 'https://api.github.com/repos/yt-dlp/yt-dlp/releases/latest'
$asset = $release.assets | Where-Object name -eq 'yt-dlp.exe' | Select-Object -First 1
$sums = $release.assets | Where-Object name -eq 'SHA2-256SUMS' | Select-Object -First 1
if (-not $asset -or -not $sums) { throw 'Release assets not found.' }
$binaryPath = Join-Path $downloadDir 'yt-dlp.exe'
Invoke-WebRequest -UseBasicParsing $asset.browser_download_url -OutFile $binaryPath
$hashFile = Join-Path $downloadDir 'yt-dlp-SHA2-256SUMS'
Invoke-WebRequest -UseBasicParsing $sums.browser_download_url -OutFile $hashFile
$hashText = [System.IO.File]::ReadAllText($hashFile)
$line = ([string]$hashText -split "`n" | Where-Object { $_ -match '\s+\*?yt-dlp\.exe\s*$' } | Select-Object -First 1)
$expected = ([regex]::Match([string]$line, '[a-fA-F0-9]{64}')).Value
if (-not $expected -or (Get-FileHash -LiteralPath $binaryPath -Algorithm SHA256).Hash -ne $expected) { throw 'yt-dlp checksum mismatch.' }
Copy-Item -LiteralPath $binaryPath -Destination (Join-Path $toolsDir 'yt-dlp.exe') -Force
# yt-dlp uses a JavaScript runtime for current YouTube extraction.
# Node is used if installed; it is not installed globally by this script.
Write-Host 'Done. FFmpeg, FFprobe and yt-dlp are in tools/.'
Write-Host 'For YouTube extraction, install Node.js 22+ or Deno if not already available.'

# skry installer for Windows.
#
#   irm https://raw.githubusercontent.com/enderkus/skry/main/install.ps1 | iex
#
# Environment variables:
#   SKRY_VERSION      release tag to install, e.g. v0.1.0 (default: latest)
#   SKRY_INSTALL_DIR  target directory (default: %LOCALAPPDATA%\Programs\skry)
#
# Downloads the release zip from GitHub, verifies its SHA-256 checksum,
# installs skry.exe and adds the directory to the user PATH.

$ErrorActionPreference = 'Stop'
$ProgressPreference = 'SilentlyContinue'

function Install-Skry {
    $repo = 'enderkus/skry'
    $base = "https://github.com/$repo/releases"
    [Net.ServicePointManager]::SecurityProtocol = [Net.ServicePointManager]::SecurityProtocol -bor [Net.SecurityProtocolType]::Tls12

    $tag = $env:SKRY_VERSION
    if (-not $tag) {
        $tag = (Invoke-RestMethod -Uri "https://api.github.com/repos/$repo/releases/latest" -Headers @{ 'User-Agent' = 'skry-installer' }).tag_name
    }
    if (-not $tag.StartsWith('v')) { $tag = "v$tag" }

    $arch = $env:PROCESSOR_ARCHITECTURE
    if ($arch -eq 'ARM64') {
        Write-Host 'skry-install: no native ARM64 build yet; installing the x86_64 build (runs under emulation)'
    } elseif ($arch -ne 'AMD64') {
        throw "unsupported architecture: $arch"
    }
    $target = 'x86_64-pc-windows-msvc'
    $name = "skry-$tag-$target"
    $url = "$base/download/$tag/$name.zip"

    $tmp = Join-Path ([IO.Path]::GetTempPath()) ("skry-" + [Guid]::NewGuid())
    New-Item -ItemType Directory -Path $tmp | Out-Null
    try {
        Write-Host "skry-install: downloading skry $tag for $target"
        $zip = Join-Path $tmp "$name.zip"
        Invoke-WebRequest -Uri $url -OutFile $zip -UseBasicParsing
        Invoke-WebRequest -Uri "$url.sha256" -OutFile "$zip.sha256" -UseBasicParsing
        $expected = ((Get-Content "$zip.sha256" -Raw).Trim() -split '\s+')[0].ToLower()
        $actual = (Get-FileHash $zip -Algorithm SHA256).Hash.ToLower()
        if ($expected -ne $actual) { throw "checksum mismatch for $name.zip" }
        Expand-Archive -Path $zip -DestinationPath $tmp -Force

        $dir = $env:SKRY_INSTALL_DIR
        if (-not $dir) { $dir = Join-Path $env:LOCALAPPDATA 'Programs\skry' }
        New-Item -ItemType Directory -Force -Path $dir | Out-Null
        Copy-Item (Join-Path $tmp "$name\skry.exe") (Join-Path $dir 'skry.exe') -Force

        $userPath = [Environment]::GetEnvironmentVariable('Path', 'User')
        $entries = @($userPath -split ';' | Where-Object { $_ })
        if ($entries -notcontains $dir) {
            [Environment]::SetEnvironmentVariable('Path', (($entries + $dir) -join ';'), 'User')
            Write-Host "skry-install: added $dir to your user PATH (open a new terminal to use it)"
        }
        if (($env:Path -split ';') -notcontains $dir) { $env:Path = "$env:Path;$dir" }

        $version = & (Join-Path $dir 'skry.exe') --version
        Write-Host "skry-install: installed $version to $dir\skry.exe"
        Write-Host 'skry-install: get started: skry --help   (or try the UI without servers: skry demo)'
    } finally {
        Remove-Item -Recurse -Force $tmp -ErrorAction SilentlyContinue
    }
}

Install-Skry

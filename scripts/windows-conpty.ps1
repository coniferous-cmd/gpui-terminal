# windows-conpty.ps1: Microsoft's pinned ConPTY beside the executable.
param([string]$Directory)
$ErrorActionPreference = 'Stop'
if (-not $Directory) {
    $target = if ($env:CARGO_TARGET_DIR) { $env:CARGO_TARGET_DIR } else {
        Join-Path $PSScriptRoot '../target'
    }
    $Directory = Join-Path $target 'debug'
}
$arch = @{ AMD64 = 'x64'; ARM64 = 'arm64'; x86 = 'x86' }[$env:PROCESSOR_ARCHITECTURE]
if (-not $arch) { throw "windows-conpty: unsupported architecture $env:PROCESSOR_ARCHITECTURE" }
$version = '1.24.261001001'
$hash = '4D6AADDC1D2385C9F5897DF28F33879F699F8F2783315D5204CF3D8C3616AC5F'
$temp = Join-Path ([IO.Path]::GetTempPath()) ([Guid]::NewGuid().ToString())
try {
    New-Item -ItemType Directory -Path $temp | Out-Null
    $archive = Join-Path $temp 'conpty.zip'
    $url = "https://api.nuget.org/v3-flatcontainer/microsoft.windows.console.conpty/$version/microsoft.windows.console.conpty.$version.nupkg"
    Write-Host "windows-conpty: downloading Microsoft.Windows.Console.ConPTY $version ($arch)"
    Invoke-WebRequest -UseBasicParsing -Uri $url -OutFile $archive
    if ((Get-FileHash -Algorithm SHA256 $archive).Hash -ne $hash) {
        throw 'windows-conpty: package SHA256 does not match the pinned release'
    }
    $unpacked = Join-Path $temp 'unpacked'
    Expand-Archive -Path $archive -DestinationPath $unpacked
    $dll = Join-Path $unpacked "runtimes/win-$arch/native/conpty.dll"
    $console = Join-Path $unpacked "build/native/runtimes/$arch/OpenConsole.exe"
    foreach ($file in @($dll, $console)) {
        if (-not (Test-Path -LiteralPath $file -PathType Leaf)) {
            throw "windows-conpty: package lacks $file"
        }
    }
    New-Item -ItemType Directory -Force -Path $Directory | Out-Null
    Copy-Item -LiteralPath $dll, $console -Destination $Directory -Force
    Copy-Item -LiteralPath (Join-Path $PSScriptRoot '../assets/licenses/conpty.txt') -Destination $Directory -Force
    Write-Host "windows-conpty: installed $version in $Directory"
} catch {
    throw "windows-conpty: runtime preparation failed: $_"
} finally {
    Remove-Item -LiteralPath $temp -Recurse -Force -ErrorAction Stop
}

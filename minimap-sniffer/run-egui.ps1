param(
    [string]$Iface = "Realtek",
    [int]$Port = 10001,
    [switch]$ListDevices,
    [string]$OfflineStreamDir = ""
)

$ErrorActionPreference = "Stop"
$Root = Split-Path -Parent $MyInvocation.MyCommand.Path

$Cargo = Get-Command cargo -ErrorAction SilentlyContinue
if (-not $Cargo) {
    $CargoPath = Join-Path $env:USERPROFILE ".cargo\bin\cargo.exe"
    if (-not (Test-Path -LiteralPath $CargoPath)) {
        throw "cargo.exe not found. Install Rust with rustup first."
    }
    $CargoExe = $CargoPath
} else {
    $CargoExe = $Cargo.Source
}

$SdkCandidates = @(@(
    $env:LIBPCAP_LIBDIR,
    (Join-Path $Root "..\..\npcap-sdk-1.16\Lib\x64"),
    "C:\Npcap-SDK\Lib\x64",
    "C:\Program Files\Npcap-SDK\Lib\x64"
) | Where-Object { $_ -and (Test-Path -LiteralPath (Join-Path $_ "wpcap.lib")) })

if ($SdkCandidates.Count -eq 0) {
    throw "wpcap.lib not found. Download/extract the Npcap SDK and set LIBPCAP_LIBDIR to its Lib\x64 folder."
}

$env:LIBPCAP_LIBDIR = (Resolve-Path -LiteralPath $SdkCandidates[0]).Path

$NpcapDllDir = "C:\Windows\System32\Npcap"
if (Test-Path -LiteralPath (Join-Path $NpcapDllDir "wpcap.dll")) {
    if (-not (($env:PATH -split ';') -contains $NpcapDllDir)) {
        $env:PATH = "$NpcapDllDir;$env:PATH"
    }
}

$CargoArgs = @("run", "--")
if ($ListDevices) {
    $CargoArgs += "--list-devices"
} elseif ($OfflineStreamDir) {
    $CargoArgs += @("--offline-stream-dir", $OfflineStreamDir)
} else {
    $CargoArgs += @("--iface", $Iface, "--port", [string]$Port)
}

Push-Location $Root
try {
    & $CargoExe @CargoArgs
} finally {
    Pop-Location
}

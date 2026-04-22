param(
    [int]$Port = 8092,
    [string]$DistDir = ".dist_local"
)

Set-StrictMode -Version Latest
$ErrorActionPreference = "Stop"

Set-Location "$PSScriptRoot\.."

# Prevent Trunk/wasm tool races from stale processes.
Get-Process trunk, wasm-opt, wasm-bindgen -ErrorAction SilentlyContinue | Stop-Process -Force

# Best-effort cleanup; locked files can be left behind by external scanners.
if (Test-Path $DistDir) {
    try { Remove-Item $DistDir -Recurse -Force } catch { Write-Warning ("Could not fully remove {0}: {1}" -f $DistDir, $_.Exception.Message) }
}

Write-Host "Serving on http://127.0.0.1:$Port using dist '$DistDir'..."
trunk serve --release --port $Port --dist $DistDir

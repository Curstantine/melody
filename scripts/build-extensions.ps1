$ErrorActionPreference = "Stop"

$repoRoot = Split-Path -Parent $PSScriptRoot
Push-Location $repoRoot
try {
    cargo build -p melody-ext-lastfm -p melody-ext-listenbrainz --target wasm32-unknown-unknown --release
    if ($LASTEXITCODE -ne 0) {
        exit $LASTEXITCODE
    }

    $targetDir = Join-Path $repoRoot "target\wasm32-unknown-unknown\release"
    Copy-Item (Join-Path $targetDir "melody_ext_lastfm.wasm") (Join-Path $repoRoot "extensions\lastfm\extension.wasm") -Force
    Copy-Item (Join-Path $targetDir "melody_ext_listenbrainz.wasm") (Join-Path $repoRoot "extensions\listenbrainz\extension.wasm") -Force

    Write-Host "Built bundled extensions:"
    Get-Item (Join-Path $repoRoot "extensions\*\extension.wasm") | Select-Object FullName, Length
}
finally {
    Pop-Location
}

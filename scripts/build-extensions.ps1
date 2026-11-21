$ErrorActionPreference = "Stop"

$repoRoot = Split-Path -Parent $PSScriptRoot
Push-Location $repoRoot
try {
    $envFile = Join-Path $repoRoot ".env"
    if (Test-Path $envFile) {
        foreach ($line in Get-Content $envFile) {
            $trimmed = $line.Trim()
            if (-not $trimmed -or $trimmed.StartsWith("#")) {
                continue
            }
            $parts = $trimmed -split "=", 2
            if ($parts.Length -eq 2) {
                $name = $parts[0].Trim()
                $val = $parts[1].Trim().Trim('"').Trim("'")
                if (-not [Environment]::GetEnvironmentVariable($name)) {
                    [Environment]::SetEnvironmentVariable($name, $val, "Process")
                }
            }
        }
    }

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

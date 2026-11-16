param(
    [Parameter(Position = 0)]
    [int]$ProcessId,

    [int]$Top = 25
)

if (-not $ProcessId) {
    $proc = Get-Process -Name "melody" -ErrorAction SilentlyContinue | Select-Object -First 1
    if (-not $proc) {
        Write-Error "No 'melody' process found. Pass -ProcessId <pid> explicitly."
        exit 1
    }
    $ProcessId = $proc.Id
}

Write-Host "=== Process Memory (PID $ProcessId) ===" -ForegroundColor Cyan
Get-Process -Id $ProcessId | Select-Object Id, ProcessName,
    @{N='WS_MB';E={[math]::Round($_.WorkingSet64/1MB,2)}},
    @{N='Private_MB';E={[math]::Round($_.PrivateMemorySize64/1MB,2)}},
    @{N='Virtual_MB';E={[math]::Round($_.VirtualMemorySize64/1MB,2)}} | Format-Table -AutoSize

Write-Host "=== Loaded Modules (PID $ProcessId) ===" -ForegroundColor Cyan
$modules = (Get-Process -Id $ProcessId).Modules |
    Sort-Object ModuleMemorySize -Descending
if ($Top -gt 0) {
    $modules = $modules | Select-Object -First $Top
}
$modules |
    Select-Object ModuleName, @{N='Mem_MB';E={[math]::Round($_.ModuleMemorySize/1MB,2)}} |
    Format-Table -AutoSize

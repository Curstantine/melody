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

$p = Get-Process -Id $ProcessId

# Query Private Working Set (unshared RAM: heap, stacks, modified data pages) via Performance Counters
$instName = (Get-CimInstance Win32_PerfFormattedData_PerfProc_Process -Filter "IDProcess = $ProcessId" -ErrorAction SilentlyContinue | Select-Object -First 1)
$privWS_MB = if ($instName) { [math]::Round($instName.WorkingSetPrivate / 1MB, 2) } else { $null }
$totalWS_MB = [math]::Round($p.WorkingSet64 / 1MB, 2)
$sharedWS_MB = if ($null -ne $privWS_MB) { [math]::Round($totalWS_MB - $privWS_MB, 2) } else { $null }
$commit_MB = [math]::Round($p.PrivateMemorySize64 / 1MB, 2)
$virtual_MB = [math]::Round($p.VirtualMemorySize64 / 1MB, 2)

Write-Host "=== Process Memory (PID $ProcessId) ===" -ForegroundColor Cyan
[PSCustomObject]@{
    Id          = $p.Id
    ProcessName = $p.ProcessName
    Total_WS_MB = $totalWS_MB
    Priv_WS_MB  = $privWS_MB    # Actual unshared physical RAM (your heap, stacks, GPU buffers)
    Shared_MB   = $sharedWS_MB  # Shared read-only DLL/EXE code pages
    Commit_MB   = $commit_MB    # Total reserved/committed private virtual memory
    Virtual_MB  = $virtual_MB
} | Format-Table -AutoSize

# Sum static virtual footprint of all loaded modules (.exe + all .dlls)
$allModules = $p.Modules
$totalModuleFootprint_MB = [math]::Round(($allModules | Measure-Object -Property ModuleMemorySize -Sum).Sum / 1MB, 2)
Write-Host "Total Loaded Modules Footprint: $totalModuleFootprint_MB MB ($($allModules.Count) modules)" -ForegroundColor DarkGray

Write-Host "`n=== Loaded Modules (PID $ProcessId) ===" -ForegroundColor Cyan
$modules = $allModules | Sort-Object ModuleMemorySize -Descending
if ($Top -gt 0) {
    $modules = $modules | Select-Object -First $Top
}
$modules |
    Select-Object ModuleName, @{N='Mem_MB';E={[math]::Round($_.ModuleMemorySize/1MB,2)}} |
    Format-Table -AutoSize

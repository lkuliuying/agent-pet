param([string]$Binary, [ValidateRange(2,30)][int]$SampleSeconds = 10)
$ErrorActionPreference = 'Stop'
$workspace = Split-Path -Parent $PSScriptRoot
if (-not $Binary) { $Binary = Join-Path $workspace 'target\release\PetDesktop.exe' }
$Binary = (Resolve-Path -LiteralPath $Binary).Path
if (Get-Process -Name PetDesktop -ErrorAction SilentlyContinue) { throw '请先关闭已运行的桌宠，避免干扰其单实例和配置。' }
$directory = Join-Path $workspace ('.runtime\smoke-' + (Get-Date -Format 'yyyyMMdd-HHmmss'))
$info = [Diagnostics.ProcessStartInfo]::new()
$info.FileName = $Binary
$info.Arguments = '--data-dir="' + $directory + '" --exit-after=' + ($SampleSeconds + 6)
$info.UseShellExecute = $false
$info.CreateNoWindow = $true
$info.RedirectStandardOutput = $true
$info.RedirectStandardError = $true
$child = [Diagnostics.Process]::Start($info)
try {
    $stdout = $child.StandardOutput.ReadToEndAsync()
    $stderr = $child.StandardError.ReadToEndAsync()
    Start-Sleep -Seconds 3
    if ($child.HasExited) { throw '桌宠过早退出，未完成启动检查。' }
    $child.Refresh()
    $initialCpu = $child.TotalProcessorTime.TotalSeconds
    $watch = [Diagnostics.Stopwatch]::StartNew()
    Start-Sleep -Seconds $SampleSeconds
    $child.Refresh()
    $watch.Stop()
    if ($child.HasExited) { throw '采样期间进程退出，资源数据无效。' }
    $report = [ordered]@{
        SampleSeconds = [Math]::Round($watch.Elapsed.TotalSeconds,3)
        LogicalProcessors = [Environment]::ProcessorCount
        CPUPercentOfMachine = [Math]::Round(($child.TotalProcessorTime.TotalSeconds - $initialCpu) / $watch.Elapsed.TotalSeconds / [Environment]::ProcessorCount * 100,3)
        WorkingSetMiB = [Math]::Round($child.WorkingSet64 / 1MB,1)
        PrivateMiB = [Math]::Round($child.PrivateMemorySize64 / 1MB,1)
    }
    if (-not $child.WaitForExit(15000)) { throw '退出超时。' }
    if ($child.ExitCode -ne 0) { throw "桌宠退出码异常：$($child.ExitCode)" }
    if ($stderr.Result.Trim()) { throw ('桌宠产生诊断错误：' + $stderr.Result) }
    $settings = Get-Content -LiteralPath (Join-Path $directory 'settings.json') -Raw -Encoding utf8 | ConvertFrom-Json
    if ($settings.version -ne 1 -or -not $settings.position) { throw '窗口位置未写入隔离配置。' }
    $report.ExitCode = $child.ExitCode
    $report.ConfigCreated = $true
    $report.ExeSHA256 = (Get-FileHash -LiteralPath $Binary -Algorithm SHA256).Hash.ToLowerInvariant()
    $report | ConvertTo-Json
} finally {
    if (-not $child.HasExited) { $child.Kill(); $child.WaitForExit() }
    $child.Dispose()
}

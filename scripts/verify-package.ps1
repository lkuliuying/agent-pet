param([Parameter(Mandatory = $true)][string]$Directory)
$ErrorActionPreference = 'Stop'
$packageRoot = (Resolve-Path -LiteralPath $Directory).Path
foreach ($line in Get-Content -LiteralPath (Join-Path $packageRoot 'SHA256SUMS.txt') -Encoding utf8) {
    if ($line -notmatch '^([0-9a-f]{64})  (.+)$') { throw 'SHA-256 清单格式错误。' }
    $expected = $Matches[1]
    $relative = $Matches[2]
    $file = [System.IO.Path]::GetFullPath((Join-Path $packageRoot $relative))
    if (-not $file.StartsWith($packageRoot + [System.IO.Path]::DirectorySeparatorChar, [StringComparison]::OrdinalIgnoreCase)) { throw '清单路径越界。' }
    if ((Get-FileHash -LiteralPath $file -Algorithm SHA256).Hash.ToLowerInvariant() -ne $expected) { throw "校验失败：$relative" }
}
$pluginRoot = Join-Path $packageRoot 'codex-marketplace\plugins\firefly-pet'
$plugin = Get-Content -LiteralPath (Join-Path $pluginRoot '.codex-plugin\plugin.json') -Raw -Encoding utf8 | ConvertFrom-Json
$hooks = Get-Content -LiteralPath (Join-Path $pluginRoot 'hooks\hooks.json') -Raw -Encoding utf8 | ConvertFrom-Json
if ($plugin.name -ne 'firefly-pet' -or @($hooks.hooks.PSObject.Properties).Count -ne 3) { throw '插件清单或事件集合错误。' }
$results = @()
foreach ($eventName in @('Stop','PermissionRequest','Interrupt')) {
    $handler = $hooks.hooks.$eventName[0].hooks[0]
    if (-not $handler.async -or $handler.timeout -ne 2) { throw 'hook 必须后台运行且超时为 2 秒。' }
    $command = $handler.commandWindows.Replace('${PLUGIN_ROOT}', $pluginRoot)
    $info = [System.Diagnostics.ProcessStartInfo]::new()
    $info.FileName = $env:ComSpec
    $info.Arguments = '/d /s /c "' + $command + '"'
    $info.WorkingDirectory = $packageRoot
    $info.UseShellExecute = $false
    $info.CreateNoWindow = $true
    $info.RedirectStandardInput = $true
    $info.RedirectStandardOutput = $true
    $info.RedirectStandardError = $true
    $watch = [Diagnostics.Stopwatch]::StartNew()
    $child = [Diagnostics.Process]::Start($info)
    try {
        $inputJson = @{ hook_event_name = $eventName; session_id = 'package-fixture'; turn_id = ('fixture-' + $eventName); last_assistant_message = '这段正文必须被丢弃' } | ConvertTo-Json -Compress
        $child.StandardInput.Write($inputJson)
        $child.StandardInput.Close()
        $stdout = $child.StandardOutput.ReadToEndAsync()
        $stderr = $child.StandardError.ReadToEndAsync()
        if (-not $child.WaitForExit(3000)) { $child.Kill(); $child.WaitForExit(); throw 'hook 进程超过 3 秒未退出。' }
        $watch.Stop()
        if ($child.ExitCode -ne 0 -or $stdout.Result.Trim() -ne '{}' -or $stderr.Result.Trim() -ne '') { throw "hook 没有中立退出：$eventName" }
        $results += [pscustomobject]@{ Event = $eventName; ExitCode = $child.ExitCode; Milliseconds = $watch.ElapsedMilliseconds; Output = $stdout.Result.Trim() }
    } finally { $child.Dispose() }
}
'所有清单文件 SHA-256 一致；以下为插件命令夹具测试，不是真实 Codex 接入验收。'
$results | Format-Table

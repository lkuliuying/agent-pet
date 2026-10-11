param([Parameter(Mandatory = $true)][string]$Directory)
$ErrorActionPreference = 'Stop'
$eventNames = @('SessionStart','UserPromptSubmit','PreToolUse','PostToolUse','Stop','PermissionRequest','Interrupt','SessionEnd')
$packageRoot = (Resolve-Path -LiteralPath $Directory).Path
$listed = @{}
foreach ($line in Get-Content -LiteralPath (Join-Path $packageRoot 'SHA256SUMS.txt') -Encoding utf8) {
    if ($line -notmatch '^([0-9a-f]{64})  (.+)$') { throw 'SHA-256 清单格式错误。' }
    $expected = $Matches[1]
    $relative = $Matches[2]
    if ($listed.ContainsKey($relative)) { throw 'SHA-256 清单包含重复路径。' }
    $listed[$relative] = $true
    $file = [System.IO.Path]::GetFullPath((Join-Path $packageRoot $relative))
    if (-not $file.StartsWith($packageRoot + [System.IO.Path]::DirectorySeparatorChar, [StringComparison]::OrdinalIgnoreCase)) { throw '清单路径越界。' }
    if ((Get-FileHash -LiteralPath $file -Algorithm SHA256).Hash.ToLowerInvariant() -ne $expected) { throw "校验失败：$relative" }
}
$required = @('PetDesktop.exe', 'codex-marketplace/.agents/plugins/marketplace.json', 'codex-marketplace/plugins/firefly-pet/.codex-plugin/plugin.json', 'codex-marketplace/plugins/firefly-pet/hooks/hooks.json', 'codex-marketplace/plugins/firefly-pet/hooks/notify.cmd', 'codex-marketplace/plugins/firefly-pet/PetDesktop.exe')
$required += @('语音资源/README.md', '语音资源/lines.json', '语音资源/generation.json', '语音资源/sources.json')
foreach ($relative in $required) {
    if (-not $listed.ContainsKey($relative) -or -not (Test-Path -LiteralPath (Join-Path $packageRoot $relative) -PathType Leaf)) {
        throw "交付包缺少必需文件或其哈希记录：$relative"
    }
}
$pluginRoot = Join-Path $packageRoot 'codex-marketplace\plugins\firefly-pet'
$marketplace = Get-Content -LiteralPath (Join-Path $packageRoot 'codex-marketplace\.agents\plugins\marketplace.json') -Raw -Encoding utf8 | ConvertFrom-Json
$plugin = Get-Content -LiteralPath (Join-Path $pluginRoot '.codex-plugin\plugin.json') -Raw -Encoding utf8 | ConvertFrom-Json
$hooks = Get-Content -LiteralPath (Join-Path $pluginRoot 'hooks\hooks.json') -Raw -Encoding utf8 | ConvertFrom-Json
if ($marketplace.name -ne 'firefly-local' -or @($marketplace.plugins).Count -ne 1 -or
    $marketplace.plugins[0].name -ne 'firefly-pet' -or $marketplace.plugins[0].source.source -ne 'local' -or
    $marketplace.plugins[0].source.path -ne './plugins/firefly-pet' -or
    $marketplace.plugins[0].policy.installation -ne 'AVAILABLE' -or
    $marketplace.plugins[0].policy.authentication -ne 'ON_INSTALL' -or
    $marketplace.plugins[0].category -ne 'Productivity') { throw '本地 marketplace 路由或安装策略错误。' }
if ($plugin.name -ne 'firefly-pet' -or $plugin.hooks -ne './hooks/hooks.json' -or
    (Compare-Object $eventNames @($hooks.hooks.PSObject.Properties.Name))) { throw '插件清单或事件集合错误。' }
if ((Get-FileHash -LiteralPath (Join-Path $packageRoot 'PetDesktop.exe')).Hash -ne
    (Get-FileHash -LiteralPath (Join-Path $pluginRoot 'PetDesktop.exe')).Hash) { throw '桌宠与插件桥接 exe 版本不一致。' }
foreach ($eventName in $eventNames) {
    $groups = @($hooks.hooks.$eventName)
    if ($groups.Count -ne 1 -or @($groups[0].hooks).Count -ne 1 -or $groups[0].matcher) { throw 'hook 不应遗漏事件或增加筛选器。' }
    $handler = $groups[0].hooks[0]
    if ($handler.type -ne 'command' -or $handler.async -ne $true -or $handler.timeout -ne 2 -or
        $handler.command -ne 'echo {}' -or
        $handler.commandWindows -ne 'cmd.exe /d /c "${PLUGIN_ROOT}\hooks\notify.cmd"') { throw 'hook 命令、后台配置或超时不符合包约定。' }
}

# 占用与桌宠相同的实例锁，夹具只进入本脚本的接收端，不能污染真实接入状态。
$sid = [Security.Principal.WindowsIdentity]::GetCurrent().User.Value
$created = $false
$instance = [Threading.Mutex]::new($false, ('Local\FireflyPet-' + $sid), [ref]$created)
if (-not $created) { $instance.Dispose(); throw '请先退出桌宠，再执行包夹具验证。' }
$results = @()
try {
    foreach ($eventName in $eventNames) {
        $handler = $hooks.hooks.$eventName[0].hooks[0]
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
        $pipe = [IO.Pipes.NamedPipeServerStream]::new(('FireflyPet-v1-' + $sid), [IO.Pipes.PipeDirection]::In, 1,
            [IO.Pipes.PipeTransmissionMode]::Message, [IO.Pipes.PipeOptions]::Asynchronous)
        $child = $null
        try {
            $connection = $pipe.WaitForConnectionAsync()
            $watch = [Diagnostics.Stopwatch]::StartNew()
            $child = [Diagnostics.Process]::Start($info)
            $inputJson = @{ hook_event_name = $eventName; session_id = 'package-fixture'; turn_id = ('fixture-' + $eventName); last_assistant_message = '这段正文必须被丢弃' } | ConvertTo-Json -Compress
            $child.StandardInput.Write($inputJson)
            $child.StandardInput.Close()
            $stdout = $child.StandardOutput.ReadToEndAsync()
            $stderr = $child.StandardError.ReadToEndAsync()
            if (-not $connection.Wait(2000)) { throw "hook 未连接到夹具接收端：$eventName" }
            $buffer = [byte[]]::new(2049)
            $read = $pipe.ReadAsync($buffer, 0, $buffer.Length)
            if (-not $read.Wait(1000) -or $read.Result -eq 0 -or $read.Result -gt 2048 -or -not $pipe.IsMessageComplete) {
                throw "hook 未送达完整状态消息：$eventName"
            }
            $wire = [Text.Encoding]::UTF8.GetString($buffer, 0, $read.Result) | ConvertFrom-Json
            if ((Compare-Object @('version','kind','session_id','turn_id','simulated','emitted_at_ms') @($wire.PSObject.Properties.Name)) -or
                $wire.version -ne 2 -or $wire.kind -ne $eventName -or $wire.session_id -ne 'package-fixture' -or
                $wire.turn_id -ne ('fixture-' + $eventName) -or $wire.simulated -ne $false) {
                throw "hook 送达的最小状态字段不正确：$eventName"
            }
            if (-not $child.WaitForExit(3000)) { $child.Kill(); $child.WaitForExit(); throw 'hook 进程超过 3 秒未退出。' }
            $watch.Stop()
            if ($child.ExitCode -ne 0 -or $stdout.Result.Trim() -ne '{}' -or $stderr.Result.Trim() -ne '') { throw "hook 没有中立退出：$eventName" }
            $results += [pscustomobject]@{ Event = $eventName; Received = $true; ExitCode = $child.ExitCode; Milliseconds = $watch.ElapsedMilliseconds; Output = $stdout.Result.Trim() }
        } finally {
            $pipe.Dispose()
            if ($child) {
                if (-not $child.HasExited) { $child.Kill(); $child.WaitForExit() }
                $child.Dispose()
            }
        }
    }
} finally { $instance.Dispose() }
'清单、路由、exe 一致性及八类消息实际送达均通过；接收端为隔离夹具，不是真实 Codex 会话或桌宠 UI 验收。'
$results | Format-Table

param([Parameter(Mandatory = $true)][string]$Directory)
$ErrorActionPreference = 'Stop'
$workspace = Split-Path -Parent $PSScriptRoot
$source = (Resolve-Path -LiteralPath $Directory).Path
$testRoot = Join-Path $workspace ('.runtime\package-check-' + [Guid]::NewGuid().ToString('N'))
$files = @('PetDesktop.exe', 'codex-marketplace/.agents/plugins/marketplace.json',
    'codex-marketplace/plugins/firefly-pet/.codex-plugin/plugin.json',
    'codex-marketplace/plugins/firefly-pet/hooks/hooks.json',
    'codex-marketplace/plugins/firefly-pet/hooks/notify.cmd',
    'codex-marketplace/plugins/firefly-pet/PetDesktop.exe')
$files += @('语音资源/README.md', '语音资源/lines.json', '语音资源/generation.json', '语音资源/sources.json')
$cases = @(
    @{ Name='中文 空格目录'; Error=$null },
    @{ Name='missing-exe'; Error='交付包缺少必需文件' },
    @{ Name='mismatched-exe'; Error='桌宠与插件桥接 exe 版本不一致' },
    @{ Name='wrong-marketplace'; Error='本地 marketplace 路由' },
    @{ Name='no-delivery'; Error='hook 未连接到夹具接收端' }
)
foreach ($case in $cases) {
    $target = Join-Path $testRoot $case.Name
    foreach ($relative in $files) {
        if ($case.Name -eq 'missing-exe' -and $relative -eq 'codex-marketplace/plugins/firefly-pet/PetDesktop.exe') { continue }
        $file = Join-Path $target $relative
        New-Item -ItemType Directory -Path (Split-Path -Parent $file) -Force | Out-Null
        Copy-Item -LiteralPath (Join-Path $source $relative) -Destination $file
    }
    switch ($case.Name) {
        'mismatched-exe' {
            # 仅追加测试夹具字节；校验必须在执行该文件前拒绝不一致的副本。
            $stream = [IO.File]::Open((Join-Path $target 'codex-marketplace/plugins/firefly-pet/PetDesktop.exe'), [IO.FileMode]::Append)
            try { $stream.WriteByte(0) } finally { $stream.Dispose() }
        }
        'wrong-marketplace' {
            $file = Join-Path $target $files[1]
            $catalog = Get-Content -LiteralPath $file -Raw -Encoding utf8 | ConvertFrom-Json
            $catalog.plugins[0].source.path = './missing-plugin'
            $catalog | ConvertTo-Json -Depth 10 | Set-Content -LiteralPath $file -Encoding utf8
        }
        'no-delivery' {
            Set-Content -LiteralPath (Join-Path $target $files[4]) -Value "@echo off`r`necho {}`r`nexit /b 0" -Encoding ascii
        }
    }
    $hashes = Get-ChildItem -LiteralPath $target -Recurse -File -Force | ForEach-Object {
        '{0}  {1}' -f (Get-FileHash -LiteralPath $_.FullName).Hash.ToLowerInvariant(), $_.FullName.Substring($target.Length + 1).Replace('\','/')
    }
    Set-Content -LiteralPath (Join-Path $target 'SHA256SUMS.txt') -Value $hashes -Encoding utf8
    $failure = $null
    try { & (Join-Path $workspace 'scripts\verify-package.ps1') -Directory $target | Out-Null }
    catch { $failure = $_.Exception.Message }
    if ($case.Error) {
        if (-not $failure -or -not $failure.Contains($case.Error)) {
            throw "反例未按预期被拒绝：$($case.Name)；实际：$failure"
        }
    } elseif ($failure) { throw "正常夹具失败：$failure" }
    [pscustomobject]@{ Case=$case.Name; Passed=$true; ExpectedRejection=[bool]$case.Error }
}
Write-Output "夹具保存在 $testRoot；没有修改输入交付包，不是真实会话验收。"

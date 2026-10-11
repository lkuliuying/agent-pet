param([switch]$SkipBuild)
$ErrorActionPreference = 'Stop'
$workspace = Split-Path -Parent $PSScriptRoot
if (-not $SkipBuild) {
    & (Join-Path $PSScriptRoot 'native-cargo.ps1') build --release --locked
    if ($LASTEXITCODE -ne 0) { throw 'Release 构建失败，未打包。' }
}
$binary = Join-Path $workspace 'target\release\PetDesktop.exe'
if (-not (Test-Path -LiteralPath $binary -PathType Leaf)) { throw '缺少 Release 程序。' }
$binaryTime = (Get-Item -LiteralPath $binary).LastWriteTimeUtc
$inputs = @('src', 'ui', 'assets') | ForEach-Object { Get-ChildItem -LiteralPath (Join-Path $workspace $_) -File -Recurse }
$inputs += @('Cargo.toml','Cargo.lock','build.rs','.cargo\config.toml') | ForEach-Object { Get-Item -LiteralPath (Join-Path $workspace $_) }
if ($inputs | Where-Object { $_.Extension -in '.rs','.slint','.png','.svg','.wav','.json','.toml','.lock' -and $_.LastWriteTimeUtc -gt $binaryTime }) {
    throw '源码或资源比 exe 新，请先重新构建。'
}
$name = 'FireflyPet-v0.1.0-windows-x64-' + (Get-Date -Format 'yyyyMMdd-HHmmss')
$output = Join-Path $workspace 'artifacts'
$stage = Join-Path $output $name
if (Test-Path -LiteralPath $stage) { throw '交付目录已存在，请稍后重试；不会覆盖旧交付。' }
New-Item -ItemType Directory -Path $stage -Force | Out-Null
Copy-Item -LiteralPath $binary -Destination (Join-Path $stage 'PetDesktop.exe')
Copy-Item -LiteralPath (Join-Path $workspace 'README.md') -Destination (Join-Path $stage '使用说明.md')
Copy-Item -LiteralPath (Join-Path $workspace 'docs') -Destination (Join-Path $stage 'docs') -Recurse
Copy-Item -LiteralPath (Join-Path $workspace 'codex-marketplace') -Destination (Join-Path $stage 'codex-marketplace') -Recurse
Copy-Item -LiteralPath $binary -Destination (Join-Path $stage 'codex-marketplace\plugins\firefly-pet\PetDesktop.exe')
$assetNotes = Join-Path $stage '角色资源'
New-Item -ItemType Directory -Path $assetNotes | Out-Null
Copy-Item -LiteralPath (Join-Path $workspace 'assets\firefly\README.md') -Destination $assetNotes
Copy-Item -LiteralPath (Join-Path $workspace 'assets\firefly\character.json') -Destination $assetNotes
$voiceNotes = Join-Path $stage '语音资源'
New-Item -ItemType Directory -Path $voiceNotes | Out-Null
foreach ($voiceFile in @('README.md', 'lines.json', 'generation.json', 'sources.json')) {
    Copy-Item -LiteralPath (Join-Path $workspace ('assets\voice\' + $voiceFile)) -Destination $voiceNotes
}

Push-Location -LiteralPath $workspace
try {
    $metadataText = & cargo metadata --format-version 1 --locked --offline --filter-platform x86_64-pc-windows-msvc
    if ($LASTEXITCODE -ne 0) { throw '无法读取依赖清单，未完成交付包。' }
    $metadata = ($metadataText -join "`n") | ConvertFrom-Json
} finally { Pop-Location }
$resolved = @{}
$metadata.resolve.nodes | ForEach-Object { $resolved[$_.id] = $true }
$packages = @($metadata.packages | Where-Object { $resolved.ContainsKey($_.id) -and $_.name -ne 'pet-desktop' } | Sort-Object name,version)
$licenses = Join-Path $stage 'licenses'
New-Item -ItemType Directory -Path $licenses | Out-Null
$notice = [System.Collections.Generic.List[string]]::new()
$notice.Add('# 第三方依赖说明')
$notice.Add('')
$notice.Add('以下来自 Cargo 锁定依赖解析结果，包含构建依赖及平台条件依赖，不表示每一项均链接到最终 exe。Slint 使用 Royalty-free 2.0 许可选项，设置页包含 AboutSlint。许可文本按包名归档；这不是项目整体开源授权声明。')
$notice.Add('')
$notice.Add('| 依赖 | 版本 | 许可标识 | 许可文本 |')
$notice.Add('| --- | --- | --- | --- |')
foreach ($package in $packages) {
    $source = Split-Path -Parent $package.manifest_path
    $destination = Join-Path $licenses ($package.name + '-' + $package.version)
    $files = @(Get-ChildItem -LiteralPath $source -File | Where-Object { $_.Name -match '^(LICENSE|COPYING|NOTICE|COPYRIGHT)' })
    $licenseDirectories = @(Get-ChildItem -LiteralPath $source -Directory | Where-Object { $_.Name -match '^(LICENSES?|LICENSING)$' })
    if ($package.license_file) {
        $extra = Join-Path $source $package.license_file
        if (Test-Path -LiteralPath $extra -PathType Leaf) { $files += Get-Item -LiteralPath $extra }
    }
    if ($files.Count -gt 0 -or $licenseDirectories.Count -gt 0) {
        New-Item -ItemType Directory -Path $destination -Force | Out-Null
        $files | Sort-Object FullName -Unique | ForEach-Object { Copy-Item -LiteralPath $_.FullName -Destination $destination }
        $licenseDirectories | ForEach-Object { Copy-Item -LiteralPath $_.FullName -Destination $destination -Recurse }
    }
    $label = if ($files.Count -gt 0 -or $licenseDirectories.Count -gt 0) { 'licenses/' + $package.name + '-' + $package.version } else { '见 crate 元数据与上游仓库' }
    $license = [string]$package.license
    $notice.Add("| $($package.name) | $($package.version) | $license | $label |")
}
Set-Content -LiteralPath (Join-Path $stage 'THIRD_PARTY_NOTICES.md') -Value $notice -Encoding utf8

$hashes = Get-ChildItem -LiteralPath $stage -File -Recurse -Force | Sort-Object FullName | ForEach-Object {
    $relative = $_.FullName.Substring($stage.Length + 1).Replace('\','/')
    '{0}  {1}' -f (Get-FileHash -LiteralPath $_.FullName -Algorithm SHA256).Hash.ToLowerInvariant(), $relative
}
Set-Content -LiteralPath (Join-Path $stage 'SHA256SUMS.txt') -Value $hashes -Encoding utf8
Add-Type -AssemblyName System.IO.Compression.FileSystem
$zip = $stage + '.zip'
[System.IO.Compression.ZipFile]::CreateFromDirectory($stage, $zip, [System.IO.Compression.CompressionLevel]::Optimal, $true)
$zipHash = (Get-FileHash -LiteralPath $zip -Algorithm SHA256).Hash.ToLowerInvariant()
Set-Content -LiteralPath ($zip + '.sha256') -Value ($zipHash + '  ' + [System.IO.Path]::GetFileName($zip)) -Encoding utf8
[pscustomobject]@{ Directory = $stage; Zip = $zip; SHA256 = $zipHash; ExeSHA256 = (Get-FileHash -LiteralPath $binary -Algorithm SHA256).Hash.ToLowerInvariant() } | Format-List

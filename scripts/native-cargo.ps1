param([Parameter(ValueFromRemainingArguments = $true)][string[]]$CargoArguments)
$ErrorActionPreference = 'Stop'
$workspace = Split-Path -Parent $PSScriptRoot
$vswhere = 'C:\Program Files (x86)\Microsoft Visual Studio\Installer\vswhere.exe'
if (-not (Test-Path -LiteralPath $vswhere)) { throw '缺少 Visual Studio C++ Build Tools。' }
$vs = & $vswhere -latest -products '*' -requires Microsoft.VisualStudio.Component.VC.Tools.x86.x64 -property installationPath
$version = (Get-Content -LiteralPath (Join-Path $vs 'VC\Auxiliary\Build\Microsoft.VCToolsVersion.default.txt')).Trim()
$vc = Join-Path $vs "VC\Tools\MSVC\$version"
$kits = 'C:\Program Files (x86)\Windows Kits\10'
$sdk = (Get-ChildItem -LiteralPath (Join-Path $kits 'Lib') -Directory | Sort-Object Name -Descending | Select-Object -First 1).Name
$env:PATH = "$(Join-Path $vc 'bin\Hostx64\x64');$(Join-Path $kits "bin\$sdk\x64");$env:PATH"
$env:LIB = "$(Join-Path $vc 'lib\x64');$(Join-Path $kits "Lib\$sdk\ucrt\x64");$(Join-Path $kits "Lib\$sdk\um\x64")"
$env:INCLUDE = "$(Join-Path $vc 'include');$(Join-Path $kits "Include\$sdk\ucrt");$(Join-Path $kits "Include\$sdk\shared");$(Join-Path $kits "Include\$sdk\um")"
Push-Location -LiteralPath $workspace
try { & cargo @CargoArguments; $result = $LASTEXITCODE } finally { Pop-Location }
exit $result

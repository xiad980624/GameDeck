# 在 Windows 10/11 + Rust MSVC + Node/pnpm 依赖已安装环境执行
$ErrorActionPreference = 'Stop'
Set-Location (Resolve-Path (Join-Path $PSScriptRoot '..'))

Write-Host '[1/3] Checking React/TypeScript…'
pnpm --filter @gamedeck/desktop check
if ($LASTEXITCODE -ne 0) { throw 'TypeScript check failed' }

Write-Host '[2/3] Building Tauri portable binary…'
pnpm build:exe
if ($LASTEXITCODE -ne 0) { throw 'Tauri build failed' }

Write-Host '[3/3] Copying to release/GameDeck.exe…'
$candidates = @(
  'target\release\gamedeck-desktop.exe',
  'target\release\GameDeck.exe',
  'apps\desktop\src-tauri\target\release\gamedeck-desktop.exe',
  'apps\desktop\src-tauri\target\release\GameDeck.exe'
)
$binary = $candidates | Where-Object { Test-Path $_ } | Select-Object -First 1
if (-not $binary) { throw 'Cannot find built EXE in known target paths. Check Cargo target-dir/build log.' }
New-Item -ItemType Directory -Force 'release' | Out-Null
Copy-Item $binary 'release\GameDeck.exe' -Force
Set-Content -LiteralPath 'release\GameDeck.portable' -Value 'Store application data beside the executable.' -Encoding utf8
Write-Host ('Built: ' + (Resolve-Path 'release\GameDeck.exe'))

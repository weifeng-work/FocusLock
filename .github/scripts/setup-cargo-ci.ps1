# CI 环境 setup-cargo-ci.ps1
# 确保使用 crates.io 官方源 + sparse 协议（避免 rsproxy 镜像超时）

# 1. 删除项目级 cargo 配置（如果存在）
$configPath = ".cargo/config.toml"
if (Test-Path $configPath) {
    Remove-Item $configPath -Force
    Write-Host "✓ 已删除项目级 cargo 配置"
}

# 2. 创建 CI 专用的 cargo 配置（sparse 协议，无需 git clone）
$ciConfigDir = ".cargo"
if (-not (Test-Path $ciConfigDir)) {
    New-Item -ItemType Directory -Path $ciConfigDir -Force | Out-Null
}

# 写入配置（使用 Out-File 避免 PowerShell here-string 在 YAML 中的转义问题）
$configContent = @"
[registries.crates-io]
protocol = "sparse"
"@ 
Set-Content -Path "$ciConfigDir/config.toml" -Value $configContent -Encoding UTF8
Write-Host "✓ 已创建 CI cargo 配置（sparse 协议）"

# 3. 验证配置
Write-Host "--- cargo 配置内容 ---"
Get-Content "$ciConfigDir/config.toml" | Write-Host
Write-Host "------------------------"

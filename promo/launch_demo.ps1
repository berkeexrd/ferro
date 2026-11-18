param([string]$statefile = "C:\ferro-demo\state_dark.json")
Get-Process ferro -ErrorAction SilentlyContinue | Stop-Process -Force
$json = [IO.File]::ReadAllText($statefile)
[IO.File]::WriteAllText("$env:APPDATA\Ferro\state.json", $json, (New-Object Text.UTF8Encoding $false))
$env:GIT_CONFIG_GLOBAL = "C:\ferro-demo\gitconfig"
Start-Process (Join-Path $PSScriptRoot "..\target\release\ferro.exe")
Start-Sleep -Seconds 4

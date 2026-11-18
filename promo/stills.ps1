. "$PSScriptRoot\drv.ps1"
$f = "$PSScriptRoot\footage"
# light
& "$PSScriptRoot\launch_demo.ps1" -statefile C:\ferro-demo\state_light.json
& "$PSScriptRoot\cap.ps1" -out "$f\light.png" -keys "jjj" -sleep 1200
# turkish
& "$PSScriptRoot\launch_demo.ps1" -statefile C:\ferro-demo\state_tr.json
& "$PSScriptRoot\cap.ps1" -out "$f\turkish.png" -keys "jjj" -sleep 1200
# conflict
& "$PSScriptRoot\launch_demo.ps1" -statefile C:\ferro-demo\state_conflict.json
& "$PSScriptRoot\cap.ps1" -out "$f\conflict.png" -keys "" -sleep 800
# operation log
& "$PSScriptRoot\launch_demo.ps1"
& "$PSScriptRoot\cap.ps1" -out "$f\tmp.png" -keys " j j jj " -sleep 600
Start-Sleep -Milliseconds 800
& "$PSScriptRoot\cap.ps1" -out "$f\oplog_pre.png" -keys "" -sleep 300
ClickAt 26 985 500
Start-Sleep -Milliseconds 1200
& "$PSScriptRoot\cap.ps1" -out "$f\oplog.png" -keys "" -sleep 300
# palette still
& "$PSScriptRoot\launch_demo.ps1"
& "$PSScriptRoot\cap.ps1" -out "$f\palette.png" -keys "^k" -sleep 900
Get-Process ferro | Stop-Process -Force

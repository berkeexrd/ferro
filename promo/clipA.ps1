. "$PSScriptRoot\drv.ps1"
Get-Process ferro -ErrorAction SilentlyContinue | Stop-Process -Force
Start-Sleep -Seconds 1
python "$PSScriptRoot\make_demo_repo.py" "C:/ferro-demo" | Out-Null
git -C C:/ferro-demo/aurora branch --set-upstream-to=origin/feature/parallel-scan | Out-Null
& "$PSScriptRoot\launch_demo.ps1"
& "$PSScriptRoot\cap.ps1" -out "$PSScriptRoot\footage\tmp.png" -keys "jjj" -sleep 900
Start-Sleep -Milliseconds 600
$rec = StartRec "$PSScriptRoot\footage\clipA_raw.mp4" 22
Start-Sleep -Milliseconds 1200
# select a range of added lines
MoveTo 700 500 300
ClickAt 494 323 700
Start-Sleep -Milliseconds 400
ShiftClickAt 494 477 600
Start-Sleep -Milliseconds 900
# stage the selection
ClickAt 1745 120 700
Start-Sleep -Milliseconds 1400
# write the commit message
ClickAt 220 817 600
Start-Sleep -Milliseconds 300
TypeText "Split the status scan across worker threads" 50
Start-Sleep -Milliseconds 700
Chord @($CTRL) $ENTER
Start-Sleep -Milliseconds 2200
# undo it
Chord @($CTRL) $Z
Start-Sleep -Milliseconds 2200
$rec.WaitForExit()

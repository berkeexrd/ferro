. "$PSScriptRoot\drv.ps1"
Get-Process ferro -ErrorAction SilentlyContinue | Stop-Process -Force
Start-Sleep -Seconds 1
python "$PSScriptRoot\make_demo_repo.py" "C:/ferro-demo" | Out-Null
git -C C:/ferro-demo/aurora branch --set-upstream-to=origin/feature/parallel-scan | Out-Null
& "$PSScriptRoot\launch_demo.ps1"
& "$PSScriptRoot\cap.ps1" -out "$PSScriptRoot\footage\tmp.png" -keys "jjj" -sleep 900
Start-Sleep -Milliseconds 600
$rec = StartRec "$PSScriptRoot\footage\clipB_raw.mp4" 30
Start-Sleep -Milliseconds 1200
# command palette
Chord @($CTRL) $K
Start-Sleep -Milliseconds 900
TypeText "switch" 90
Start-Sleep -Milliseconds 1400
Tap $ESC
Start-Sleep -Milliseconds 700
# branch switcher with a dirty tree
Chord @($CTRL) $B
Start-Sleep -Milliseconds 1100
TypeText "fix" 120
Start-Sleep -Milliseconds 900
Tap $ENTER
Start-Sleep -Milliseconds 2200
Tap $ESC
Start-Sleep -Milliseconds 700
# history with the graph
Chord @($CTRL) $TWO
Start-Sleep -Milliseconds 1600
ClickAt 360 337 700
Start-Sleep -Milliseconds 1800
ClickAt 360 425 600
Start-Sleep -Milliseconds 1800
ClickAt 360 513 600
Start-Sleep -Milliseconds 1800
# branches and stash pages
Chord @($CTRL) $THREE
Start-Sleep -Milliseconds 2200
ClickAt 26 252 700
Start-Sleep -Milliseconds 700
ClickAt 250 190 500
Start-Sleep -Milliseconds 2200
Chord @($CTRL) $ONE
Start-Sleep -Milliseconds 1500
$rec.WaitForExit()

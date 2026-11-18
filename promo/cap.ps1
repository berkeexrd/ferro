param([string]$out,[string]$keys="",[int]$w=1920,[int]$h=1040,[int]$x=0,[int]$y=0,[int]$sleep=1500)
Add-Type -AssemblyName System.Windows.Forms, System.Drawing
Add-Type @"
using System; using System.Runtime.InteropServices;
public class W2 { [DllImport("user32.dll")] public static extern bool SetForegroundWindow(IntPtr h); [DllImport("user32.dll")] public static extern bool MoveWindow(IntPtr h,int x,int y,int w,int ht,bool r); [DllImport("user32.dll")] public static extern bool ShowWindow(IntPtr h,int c); [DllImport("user32.dll")] public static extern bool SetWindowPos(IntPtr h,IntPtr a,int x,int y,int w,int ht,uint f); }
"@
$p = Get-Process ferro | Select -First 1
[W2]::ShowWindow($p.MainWindowHandle, 9) | Out-Null
[W2]::SetWindowPos($p.MainWindowHandle, [IntPtr](-1), $x,$y,$w,$h, 0x40) | Out-Null
[W2]::SetForegroundWindow($p.MainWindowHandle) | Out-Null
Start-Sleep -Milliseconds 700
if ($keys -ne "") { [System.Windows.Forms.SendKeys]::SendWait($keys); Start-Sleep -Milliseconds $sleep }
$bmp = New-Object System.Drawing.Bitmap $w, $h
$g = [System.Drawing.Graphics]::FromImage($bmp)
$g.CopyFromScreen($x, $y, 0, 0, $bmp.Size)
$bmp.Save($out)

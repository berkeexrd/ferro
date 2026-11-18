# Input driver for screen recordings: smooth cursor, hardware-scancode typing.
Add-Type @"
using System; using System.Runtime.InteropServices;
public class DRV {
  [DllImport("user32.dll")] public static extern bool SetCursorPos(int x,int y);
  [DllImport("user32.dll")] public static extern bool GetCursorPos(out POINT p);
  [DllImport("user32.dll")] public static extern void mouse_event(uint f,uint dx,uint dy,uint d,UIntPtr e);
  [DllImport("user32.dll")] public static extern void keybd_event(byte vk, byte sc, uint f, UIntPtr e);
  public struct POINT { public int X, Y; }
}
"@
$global:SC = @{ 'a'=0x1E;'s'=0x1F;'d'=0x20;'f'=0x21;'g'=0x22;'h'=0x23;'j'=0x24;'k'=0x25;'l'=0x26;'q'=0x10;'w'=0x11;'e'=0x12;'r'=0x13;'t'=0x14;'y'=0x15;'u'=0x16;'o'=0x18;'p'=0x19;'z'=0x2C;'x'=0x2D;'c'=0x2E;'v'=0x2F;'b'=0x30;'n'=0x31;'m'=0x32;' '=0x39;'i'=0x28;'-'=0x35;'.'=0x34;',' =0x33 }
function Ease($t){ return $t*$t*(3-2*$t) }
function MoveTo([int]$x,[int]$y,[int]$ms=450){
  $p = New-Object DRV+POINT; [DRV]::GetCursorPos([ref]$p) | Out-Null
  $sx=$p.X; $sy=$p.Y; $steps=[Math]::Max(8,[int]($ms/10))
  for($i=1;$i -le $steps;$i++){ $t=Ease($i/$steps); [DRV]::SetCursorPos([int]($sx+($x-$sx)*$t),[int]($sy+($y-$sy)*$t)) | Out-Null; Start-Sleep -Milliseconds 10 }
}
function Click(){ [DRV]::mouse_event(0x02,0,0,0,[UIntPtr]::Zero); Start-Sleep -Milliseconds 60; [DRV]::mouse_event(0x04,0,0,0,[UIntPtr]::Zero); Start-Sleep -Milliseconds 120 }
function ClickAt([int]$x,[int]$y,[int]$ms=450){ MoveTo $x $y $ms; Start-Sleep -Milliseconds 120; Click }
function ShiftClickAt([int]$x,[int]$y,[int]$ms=450){ MoveTo $x $y $ms; [DRV]::keybd_event(0x10,0x2A,0,[UIntPtr]::Zero); Start-Sleep -Milliseconds 60; Click; [DRV]::keybd_event(0x10,0x2A,2,[UIntPtr]::Zero) }
function KeyDown([byte]$sc){ [DRV]::keybd_event(0,$sc,8,[UIntPtr]::Zero) }
function KeyUp([byte]$sc){ [DRV]::keybd_event(0,$sc,10,[UIntPtr]::Zero) }
function Tap([byte]$sc){ KeyDown $sc; Start-Sleep -Milliseconds 35; KeyUp $sc }
function Chord([byte[]]$mods,[byte]$sc){ foreach($m in $mods){KeyDown $m}; Start-Sleep -Milliseconds 40; Tap $sc; foreach($m in $mods){KeyUp $m}; Start-Sleep -Milliseconds 120 }
function TypeText([string]$s,[int]$delay=55){
  foreach($ch in $s.ToCharArray()){
    $lo=[string]$ch.ToString().ToLower(); $up = ($ch -cmatch '[A-Z]')
    if($up){ KeyDown 0x2A }
    if($SC.ContainsKey($lo)){ Tap ([byte]$SC[$lo]) }
    if($up){ KeyUp 0x2A }
    Start-Sleep -Milliseconds ($delay + (Get-Random -Minimum 0 -Maximum 40))
  }
}
$CTRL=0x1D; $SHIFT=0x2A; $ENTER=0x1C; $ESC=0x01; $SPACE=0x39; $K=0x25; $B=0x30; $Z=0x2C; $ONE=0x02; $TWO=0x03; $THREE=0x04; $DOWN=0x50; $UP=0x48
function StartRec([string]$out,[int]$secs,[int]$w=1920,[int]$h=1040){
  $a = "-y -f gdigrab -framerate 60 -offset_x 0 -offset_y 0 -video_size ${w}x${h} -draw_mouse 1 -i desktop -t $secs -c:v libx264 -preset ultrafast -crf 12 -pix_fmt yuv420p `"$out`""
  return Start-Process -FilePath ffmpeg -ArgumentList $a -PassThru -WindowStyle Hidden
}

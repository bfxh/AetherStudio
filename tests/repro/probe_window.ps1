Add-Type @"
using System;
using System.Runtime.InteropServices;
using System.Text;
public class W2 {
    [DllImport("user32.dll", CharSet=CharSet.Unicode)] public static extern int GetClassName(IntPtr h, StringBuilder s, int n);
    [DllImport("user32.dll")] public static extern bool GetWindowRect(IntPtr h, out RECT r);
    [StructLayout(LayoutKind.Sequential)] public struct RECT { public int L, T, R, B; }
}
"@
$p = Get-Process aether-app -ErrorAction SilentlyContinue | Select-Object -First 1
if (-not $p) { Write-Host "进程不存在"; exit 1 }
$h = $p.MainWindowHandle
Write-Host "PID=$($p.Id) HWND=$h Title='$($p.MainWindowTitle)'"
if ($h -ne [IntPtr]::Zero) {
    $sb = New-Object System.Text.StringBuilder 256
    [W2]::GetClassName($h, $sb) | Out-Null
    Write-Host "Class=$($sb.ToString())"
    $r = New-Object W2+RECT
    [W2]::GetWindowRect($h, [ref]$r) | Out-Null
    Write-Host "Rect=($($r.L),$($r.T))-($($r.R),$($r.B))"
}
# 列出所有 aether-app 的顶级窗口（枚举）
Add-Type @"
using System;
using System.Collections.Generic;
using System.Runtime.InteropServices;
using System.Text;
public class EnumW {
    delegate bool EnumProc(IntPtr h, IntPtr l);
    [DllImport("user32.dll")] static extern bool EnumWindows(EnumProc e, IntPtr l);
    [DllImport("user32.dll")] static extern uint GetWindowThreadProcessId(IntPtr h, out uint pid);
    [DllImport("user32.dll", CharSet=CharSet.Unicode)] static extern int GetWindowText(IntPtr h, StringBuilder s, int n);
    [DllImport("user32.dll", CharSet=CharSet.Unicode)] static extern int GetClassName(IntPtr h, StringBuilder s, int n);
    [DllImport("user32.dll")] static extern bool IsWindowVisible(IntPtr h);
    public static void List(uint pid) {
        EnumWindows((h, l) => {
            uint p2; GetWindowThreadProcessId(h, out p2);
            if (p2 == pid) {
                var t = new StringBuilder(256); GetWindowText(h, t, 256);
                var c = new StringBuilder(256); GetClassName(h, c, 256);
                Console.WriteLine("hwnd={0} visible={1} class={2} title={3}", h, IsWindowVisible(h), c, t);
            }
            return true;
        }, IntPtr.Zero);
    }
}
"@
[EnumW]::List([uint32]$p.Id)

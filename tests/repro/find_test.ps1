Add-Type 'using System; using System.Runtime.InteropServices; public class FW { [DllImport("user32.dll", CharSet=CharSet.Unicode)] public static extern IntPtr FindWindowW(string c, string t); }'
$h = [FW]::FindWindowW('AetherEditor', $null)
Write-Host "FindWindowW => $h"
Get-Process aether-app -ErrorAction SilentlyContinue | Select-Object Id, MainWindowHandle, MainWindowTitle

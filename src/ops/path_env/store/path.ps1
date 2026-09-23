$ErrorActionPreference = 'Stop'
[Console]::InputEncoding = [System.Text.UTF8Encoding]::new($false)
[Console]::OutputEncoding = [System.Text.UTF8Encoding]::new($false)
try {
    $request = [Console]::In.ReadToEnd() | ConvertFrom-Json
    $subkey = if ($request.scope -eq 'User') { 'Environment' } else { 'SYSTEM\CurrentControlSet\Control\Session Manager\Environment' }
    $hive = if ($request.scope -eq 'User') { [Microsoft.Win32.Registry]::CurrentUser } else { [Microsoft.Win32.Registry]::LocalMachine }
    $key = $hive.OpenSubKey($subkey, $request.action -eq 'write')
    $value = if ($key) { [string]$key.GetValue('Path', '', [Microsoft.Win32.RegistryValueOptions]::DoNotExpandEnvironmentNames) } else { '' }
    $kind = if ($key -and $key.GetValueNames() -contains 'Path') { $key.GetValueKind('Path').ToString() } else { 'ExpandString' }
    $revision = $kind + '|' + $value
    if ($request.action -eq 'write') {
        if ($revision -cne $request.expected) { throw 'PATH changed outside FFM. Reopen the editor before saving.' }
        if (-not $key) { $key = $hive.CreateSubKey($subkey) }
        $valueKind = if ($kind -eq 'ExpandString' -or $request.value.Contains('%')) { [Microsoft.Win32.RegistryValueKind]::ExpandString } else { [Microsoft.Win32.RegistryValueKind]::String }
        $key.SetValue('Path', [string]$request.value, $valueKind)
        $key.Flush()
        $value = [string]$request.value
        $revision = $valueKind.ToString() + '|' + $value
    }
    if ($key) { $key.Dispose() }
    @{ value = $value; revision = $revision; location = $hive.Name + '\' + $subkey } | ConvertTo-Json -Compress
    if ($request.action -eq 'write') {
      try {
        Add-Type -TypeDefinition 'using System; using System.Runtime.InteropServices; public static class FfmEnvironment { [DllImport("user32.dll", CharSet=CharSet.Unicode)] public static extern IntPtr SendMessageTimeout(IntPtr h, uint m, UIntPtr w, string l, uint f, uint t, out UIntPtr r); }'
        $result = [UIntPtr]::Zero
        [void][FfmEnvironment]::SendMessageTimeout([IntPtr]65535, 26, [UIntPtr]::Zero, 'Environment', 2, 1000, [ref]$result)
      } catch { }
    }
} catch {
    [Console]::Error.WriteLine('Cannot access PATH. System PATH requires administrator rights. ' + $_.Exception.Message)
    exit 1
}

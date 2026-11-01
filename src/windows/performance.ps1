$os = Get-CimInstance Win32_OperatingSystem
$cpu = Get-CimInstance Win32_Processor | Measure-Object -Property LoadPercentage -Average
$drives = Get-CimInstance Win32_LogicalDisk -Filter "DriveType=3"
$totalMem = [math]::Round($os.TotalVisibleMemorySize / 1024, 1)
$freeMem = [math]::Round($os.FreePhysicalMemory / 1024, 1)
"Computer: $env:COMPUTERNAME"
"CPU Load: $([math]::Round($cpu.Average, 1))%"
"Memory Free: $freeMem MB / $totalMem MB"
foreach ($drive in $drives) {
    $freeGb = [math]::Round($drive.FreeSpace / 1GB, 2)
    $sizeGb = [math]::Round($drive.Size / 1GB, 2)
    $usedPct = if ($drive.Size -gt 0) { [math]::Round((($drive.Size - $drive.FreeSpace) / $drive.Size) * 100, 1) } else { 0 }
    "{0} Used: {1}% Free: {2} GB / {3} GB" -f $drive.DeviceID, $usedPct, $freeGb, $sizeGb
}

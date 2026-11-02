$events = Get-WinEvent -FilterHashtable @{LogName='System'; Level=@(1,2,3)} -MaxEvents 20 |
    Select-Object TimeCreated, LevelDisplayName, ProviderName, Id, Message
foreach ($event in $events) {
    $message = ($event.Message -replace '\r?\n', ' ').Trim()
    "{0:u} [{1}] {2} ({3}) {4}" -f $event.TimeCreated, $event.LevelDisplayName, $event.ProviderName, $event.Id, $message
}

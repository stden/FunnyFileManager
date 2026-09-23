# Chocolatey writes into the machine-wide package store, so it needs an
# elevated console. Start-Process -Verb RunAs raises the UAC prompt, -Wait
# blocks until the upgrade finishes, and -PassThru returns the process object
# whose ExitCode the caller reports.
#
# `-y` answers every confirmation up front: the menu suspends the TUI and hands
# the terminal over, so a prompt here would wait for input nobody can give.
#
# {path} is substituted with the discovered choco.exe. The caller has already
# doubled any embedded apostrophe, so it is safe inside a single-quoted literal.
try {
    $process = Start-Process -FilePath '{path}' -ArgumentList 'upgrade', 'all', '-y' -Verb RunAs -Wait -PassThru
    exit $process.ExitCode
} catch {
    Write-Error $_
    exit 1
}

# Chocolatey writes into the machine-wide package store, so it needs an
# elevated console. Start-Process -Verb RunAs raises the UAC prompt, -Wait
# blocks until the upgrade finishes, and -PassThru returns the process object
# whose ExitCode the caller reports.
#
# `-y` answers every confirmation up front: the menu suspends the TUI and hands
# the terminal over, so a prompt here would wait for input nobody can give.
#
# The caller puts the discovered choco.exe in -FilePath below, with any embedded
# apostrophe already doubled, so it is safe inside a single-quoted literal. The
# argument list must match Manager::commands (a test in ops/packages checks it).
try {
    $process = Start-Process -FilePath '__FFM_EXECUTABLE__' -ArgumentList 'upgrade', 'all', '-y' -Verb RunAs -Wait -PassThru
    exit $process.ExitCode
} catch {
    Write-Error $_
    exit 1
}

//! Purpose: Unit tests for Toolhelp32 snapshot and memory query fallbacks.

use windows_sys::Win32::System::Diagnostics::ToolHelp::TH32CS_SNAPMODULE;

use super::*;

#[test]
fn snapshot_and_memory_helpers_handle_invalid_flags_and_null_handles() {
    assert!(open_snapshot(TH32CS_SNAPMODULE, u32::MAX).is_err());
    let mut owners = WinOwnerCache::default();
    let (procs, unreadable) = collect_snapshot_processes(std::ptr::null_mut(), &mut owners);
    assert!(procs.is_empty());
    assert_eq!(unreadable, 0);
    assert!(query_memory(std::ptr::null_mut()).is_none());
}

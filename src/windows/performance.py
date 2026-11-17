"""Print a read-only Windows performance report with native APIs."""

import ctypes
from ctypes import wintypes
import os
import socket
import time


class MemoryStatus(ctypes.Structure):
    _fields_ = [("length", wintypes.DWORD), ("load", wintypes.DWORD)] + [
        (name, ctypes.c_ulonglong) for name in (
            "total", "available", "total_page", "available_page", "total_virtual", "available_virtual", "extended"
        )
    ]


def cpu_times(kernel):
    idle, system, user = (wintypes.FILETIME() for _ in range(3))
    if not kernel.GetSystemTimes(ctypes.byref(idle), ctypes.byref(system), ctypes.byref(user)):
        raise ctypes.WinError(ctypes.get_last_error())
    return tuple(value.dwLowDateTime + (value.dwHighDateTime << 32) for value in (idle, system, user))


def format_report(computer, cpu, total_mb, free_mb, drives):
    lines = [f"Computer: {computer}", f"CPU Load: {cpu:.1f}%", f"Memory Free: {free_mb:.1f} MB / {total_mb:.1f} MB"]
    for drive, free, total in drives:
        used = (total - free) / total * 100 if total else 0
        lines.append(f"{drive} Used: {used:.1f}% Free: {free / 2**30:.1f} GB / {total / 2**30:.1f} GB")
    return "\n".join(lines)


def report():
    kernel = ctypes.WinDLL("kernel32", use_last_error=True)
    kernel.GetSystemTimes.argtypes = [ctypes.POINTER(wintypes.FILETIME)] * 3
    kernel.GlobalMemoryStatusEx.argtypes = [ctypes.POINTER(MemoryStatus)]
    kernel.GetDriveTypeW.argtypes = [wintypes.LPCWSTR]
    kernel.GetDiskFreeSpaceExW.argtypes = [wintypes.LPCWSTR] + [ctypes.POINTER(ctypes.c_ulonglong)] * 3
    before = cpu_times(kernel)
    time.sleep(0.25)
    after = cpu_times(kernel)
    idle, system, user = (end - start for start, end in zip(before, after))
    total_time = system + user
    cpu = 100 * (total_time - idle) / total_time if total_time else 0
    memory = MemoryStatus()
    memory.length = ctypes.sizeof(memory)
    if not kernel.GlobalMemoryStatusEx(ctypes.byref(memory)):
        raise ctypes.WinError(ctypes.get_last_error())
    drives = []
    mask = kernel.GetLogicalDrives()
    for index in range(26):
        drive = chr(65 + index) + ":\\"
        if not mask & (1 << index) or kernel.GetDriveTypeW(drive) != 3:
            continue
        available, total, free = (ctypes.c_ulonglong() for _ in range(3))
        if not kernel.GetDiskFreeSpaceExW(drive, ctypes.byref(available), ctypes.byref(total), ctypes.byref(free)):
            raise ctypes.WinError(ctypes.get_last_error())
        drives.append((drive[:2], free.value, total.value))
    return format_report(os.getenv("COMPUTERNAME", socket.gethostname()), cpu,
                         memory.total / 2**20, memory.available / 2**20, drives)


if __name__ == "__main__":
    print(report())

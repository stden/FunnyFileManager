"""Run the fixed Chocolatey upgrade through the Windows UAC prompt."""

import ctypes
from ctypes import wintypes
import subprocess
import sys

UPGRADE_ARGUMENTS = ["upgrade", "all", "-y"]


class ShellExecuteInfo(ctypes.Structure):
    _fields_ = [("cbSize", wintypes.DWORD), ("fMask", wintypes.ULONG), ("hwnd", wintypes.HWND),
                ("lpVerb", wintypes.LPCWSTR), ("lpFile", wintypes.LPCWSTR), ("lpParameters", wintypes.LPCWSTR),
                ("lpDirectory", wintypes.LPCWSTR), ("nShow", ctypes.c_int), ("hInstApp", wintypes.HINSTANCE),
                ("lpIDList", ctypes.c_void_p), ("lpClass", wintypes.LPCWSTR), ("hkeyClass", wintypes.HKEY),
                ("dwHotKey", wintypes.DWORD), ("hIcon", wintypes.HANDLE), ("hProcess", wintypes.HANDLE)]


def elevate(executable, arguments):
    shell = ctypes.WinDLL("shell32", use_last_error=True)
    kernel = ctypes.WinDLL("kernel32", use_last_error=True)
    shell.ShellExecuteExW.argtypes = [ctypes.POINTER(ShellExecuteInfo)]
    kernel.WaitForSingleObject.argtypes = [wintypes.HANDLE, wintypes.DWORD]
    kernel.GetExitCodeProcess.argtypes = [wintypes.HANDLE, ctypes.POINTER(wintypes.DWORD)]
    kernel.CloseHandle.argtypes = [wintypes.HANDLE]
    info = ShellExecuteInfo()
    info.cbSize, info.fMask, info.lpVerb = ctypes.sizeof(info), 0x40, "runas"
    info.lpFile, info.lpParameters, info.nShow = executable, subprocess.list2cmdline(arguments), 1
    if not shell.ShellExecuteExW(ctypes.byref(info)):
        raise ctypes.WinError(ctypes.get_last_error())
    if not info.hProcess:
        raise OSError("The UAC request returns no process handle.")
    try:
        if kernel.WaitForSingleObject(info.hProcess, 0xFFFFFFFF) != 0:
            raise ctypes.WinError(ctypes.get_last_error())
        code = wintypes.DWORD()
        if not kernel.GetExitCodeProcess(info.hProcess, ctypes.byref(code)):
            raise ctypes.WinError(ctypes.get_last_error())
        return code.value
    finally:
        kernel.CloseHandle(info.hProcess)


def run(executable, arguments, launcher=None):
    if list(arguments) != UPGRADE_ARGUMENTS or not executable or "\0" in executable:
        raise ValueError("The elevated helper accepts only the fixed Chocolatey upgrade.")
    return (launcher or elevate)(executable, list(arguments))


def main(argv=None):
    args = sys.argv[1:] if argv is None else argv
    try:
        if not args:
            raise ValueError("The elevated helper requires the discovered Chocolatey executable.")
        return run(args[0], args[1:])
    except (OSError, ValueError) as error:
        print(error, file=sys.stderr)
        return 1


if __name__ == "__main__":
    raise SystemExit(main())

"""Print recent critical, error, and warning events with the Windows Event Log API."""

import ctypes
from ctypes import wintypes
from datetime import datetime
import re
import xml.etree.ElementTree as ET

NS = {"e": "http://schemas.microsoft.com/win/2004/08/events/event"}
LEVELS = {1: "Critical", 2: "Error", 3: "Warning"}


def format_event(xml, message):
    system = ET.fromstring(xml).find("e:System", NS)
    provider = system.find("e:Provider", NS).attrib["Name"]
    identifier = system.findtext("e:EventID", namespaces=NS)
    level = LEVELS.get(int(system.findtext("e:Level", namespaces=NS)), "Unknown")
    stamp = system.find("e:TimeCreated", NS).attrib["SystemTime"]
    stamp = re.sub(r"(\.\d{6})\d+", r"\1", stamp).replace("Z", "+00:00")
    date = datetime.fromisoformat(stamp).strftime("%Y-%m-%d %H:%M:%SZ")
    message = re.sub(r"\r?\n", " ", message).strip()
    return f"{date} [{level}] {provider} ({identifier}) {message}"


class EventLog:
    def __init__(self):
        self.api = ctypes.WinDLL("wevtapi", use_last_error=True)
        handle, number, text, pointer = wintypes.HANDLE, wintypes.DWORD, wintypes.LPCWSTR, ctypes.c_void_p
        self.api.EvtQuery.argtypes = [handle, text, text, number]
        self.api.EvtQuery.restype = handle
        self.api.EvtNext.argtypes = [handle, number, ctypes.POINTER(handle), number, number, ctypes.POINTER(number)]
        self.api.EvtRender.argtypes = [handle, handle, number, number, pointer,
                                      ctypes.POINTER(number), ctypes.POINTER(number)]
        self.api.EvtClose.argtypes = [handle]
        self.api.EvtOpenPublisherMetadata.argtypes = [handle, text, text, number, number]
        self.api.EvtOpenPublisherMetadata.restype = handle
        self.api.EvtFormatMessage.argtypes = [handle, handle, number, number, pointer, number, number,
                                             wintypes.LPWSTR, ctypes.POINTER(number)]

    def xml(self, event):
        size, count = wintypes.DWORD(), wintypes.DWORD()
        self.api.EvtRender(None, event, 1, 0, None, ctypes.byref(size), ctypes.byref(count))
        if not size.value:
            raise ctypes.WinError(ctypes.get_last_error())
        buffer = ctypes.create_unicode_buffer((size.value + 1) // 2)
        if not self.api.EvtRender(None, event, 1, ctypes.sizeof(buffer), buffer, ctypes.byref(size), ctypes.byref(count)):
            raise ctypes.WinError(ctypes.get_last_error())
        return buffer.value

    def message(self, event, provider):
        metadata = self.api.EvtOpenPublisherMetadata(None, provider, None, 0, 0)
        if not metadata:
            return ""
        try:
            size = wintypes.DWORD()
            self.api.EvtFormatMessage(metadata, event, 0, 0, None, 1, 0, None, ctypes.byref(size))
            if not size.value:
                return ""
            buffer = ctypes.create_unicode_buffer(size.value)
            if not self.api.EvtFormatMessage(metadata, event, 0, 0, None, 1,
                                              len(buffer), buffer, ctypes.byref(size)):
                return ""
            return buffer.value
        finally:
            self.api.EvtClose(metadata)

    def report(self):
        query = self.api.EvtQuery(None, "System", "*[System[(Level=1 or Level=2 or Level=3)]]", 0x201)
        if not query:
            raise ctypes.WinError(ctypes.get_last_error())
        events = (wintypes.HANDLE * 20)()
        count = wintypes.DWORD()
        try:
            if not self.api.EvtNext(query, 20, events, 0xFFFFFFFF, 0, ctypes.byref(count)):
                if ctypes.get_last_error() == 259:
                    return ""
                raise ctypes.WinError(ctypes.get_last_error())
            lines = []
            for event in events[:count.value]:
                xml = self.xml(event)
                provider = ET.fromstring(xml).find("e:System/e:Provider", NS).attrib["Name"]
                lines.append(format_event(xml, self.message(event, provider)))
            return "\n".join(lines)
        finally:
            for event in events[:count.value]:
                self.api.EvtClose(event)
            self.api.EvtClose(query)


if __name__ == "__main__":
    print(EventLog().report())

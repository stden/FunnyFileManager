# Purpose: Deterministic stdio LSP peer used only by isolated process tests.
import json
import os
import sys
import time


def send(value):
    body = json.dumps(dict(jsonrpc="2.0", **value)).encode()
    sys.stdout.buffer.write(f"Content-Length: {len(body)}\r\n\r\n".encode() + body)
    sys.stdout.buffer.flush()


def read():
    length = 0
    while True:
        line = sys.stdin.buffer.readline()
        if not line:
            return None
        if line == b"\r\n":
            return json.loads(sys.stdin.buffer.read(length))
        if line.lower().startswith(b"content-length:"):
            length = int(line.split(b":")[1])


mode = sys.argv[1] if len(sys.argv) > 1 else "normal"
with open("pid", "w") as output:
    output.write(str(os.getpid()))
while (message := read()) is not None:
    method = message.get("method", "")
    with open("methods", "a", newline="\n") as output:
        output.write(method + "\n")
    if method == "initialize":
        if mode == "busy":
            time.sleep(0.3)
        if mode == "crash":
            sys.exit(3)
        if mode == "malformed":
            sys.stdout.buffer.write(b"Content-Length: invalid\r\n\r\n")
            sys.stdout.buffer.flush()
            continue
        send(dict(id=message["id"], result=dict(capabilities=dict(
            textDocumentSync=2, hoverProvider=(mode != "unsupported"), definitionProvider=True,
            completionProvider={}))))
    elif method in ("textDocument/didOpen", "textDocument/didChange"):
        doc = message["params"]["textDocument"]
        send(dict(method="textDocument/publishDiagnostics", params=dict(
            uri=doc["uri"], version=doc["version"], diagnostics=[dict(
                range=dict(start=dict(line=0, character=0), end=dict(line=0, character=2)),
                severity=2, message="Test warning")])) )
    elif method == "textDocument/hover":
        send(dict(id=message["id"], result=dict(contents=dict(kind="markdown", value="`symbol`"))))
    elif method == "textDocument/completion":
        send(dict(id=message["id"], result=[dict(label="completed", detail="function", documentation="Test documentation", kind=3)]))
    elif method == "textDocument/definition":
        send(dict(id=message["id"], result=dict(uri=message["params"]["textDocument"]["uri"], range=dict(start=dict(line=0, character=0), end=dict(line=0, character=2)))))
    elif method == "shutdown" and mode != "hang":
        send(dict(id=message["id"], result=None))
    elif method == "exit" and mode != "hang":
        break

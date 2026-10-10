"""A real stdio subprocess, deliberately sends stale work and UTF-16 diagnostics."""
import json
import sys


def send(value):
    data = json.dumps(dict(jsonrpc="2.0", **value), ensure_ascii=False).encode()
    sys.stdout.buffer.write(f"Content-Length: {len(data)}\r\n\r\n".encode() + data)
    sys.stdout.buffer.flush()


def read():
    length = None
    while True:
        line = sys.stdin.buffer.readline()
        if not line:
            return None
        if line in (b"\r\n", b"\n"):
            break
        if line.lower().startswith(b"content-length:"):
            length = int(line.split(b":", 1)[1])
    return json.loads(sys.stdin.buffer.read(length))


delayed = None
docs = {}
initialized = False
while (msg := read()) is not None:
    method = msg.get("method")
    params = msg.get("params", {})
    request_id = msg.get("id")
    if method == "initialize":
        assert params["capabilities"]["general"]["positionEncodings"] == ["utf-16"]
        send({"id": request_id, "result": {"capabilities": {
            "positionEncoding": "utf-16", "textDocumentSync": 2,
            "completionProvider": {}, "hoverProvider": True,
            "definitionProvider": True, "referencesProvider": True,
            "renameProvider": True, "documentFormattingProvider": True,
            "codeActionProvider": True, "documentSymbolProvider": True
        }, "serverInfo": {"name": "OpenIDE protocol fixture"}}})
    elif method == "initialized":
        initialized = True
    elif method == "textDocument/didOpen":
        assert initialized
        doc = params["textDocument"]
        docs[doc["uri"]] = doc
        send({"method": "textDocument/publishDiagnostics", "params": {
            "uri": doc["uri"], "version": doc["version"], "diagnostics": [{
                "range": {"start": {"line": 0, "character": 1}, "end": {"line": 0, "character": 3}},
                "severity": 2, "message": "non-BMP fixture"
            }]
        }})
    elif method == "textDocument/didChange":
        doc = params["textDocument"]
        docs[doc["uri"]]["version"] = doc["version"]
        docs[doc["uri"]]["text"] = params["contentChanges"][0]["text"]
        if delayed is not None:
            send({"id": delayed, "result": {"contents": "STALE"}})
            delayed = None
        send({"method": "textDocument/publishDiagnostics", "params": {
            "uri": doc["uri"], "version": doc["version"] - 1,
            "diagnostics": [{"range": {"start": {"line": 0, "character": 0}, "end": {"line": 0, "character": 1}}, "message": "STALE"}]
        }})
        send({"method": "textDocument/publishDiagnostics", "params": {"uri": doc["uri"], "version": doc["version"], "diagnostics": []}})
    elif method == "textDocument/didClose":
        docs.pop(params["textDocument"]["uri"], None)
    elif method == "$/cancelRequest":
        pass  # A cancellation does not guarantee the server stops its computation.
    elif method == "shutdown":
        send({"id": request_id, "result": None})
    elif method == "exit":
        break
    elif method and method.startswith("textDocument/"):
        uri = params["textDocument"]["uri"]
        assert uri in docs
        if method == "textDocument/hover" and params["position"]["character"] == 0:
            delayed = request_id
            continue
        location = {"uri": uri, "range": {"start": {"line": 0, "character": 1}, "end": {"line": 0, "character": 3}}}
        edit = {"range": location["range"], "newText": "replacement"}
        results = {
            "textDocument/completion": {"isIncomplete": False, "items": [{"label": "example", "textEdit": edit}]},
            "textDocument/hover": {"contents": {"kind": "markdown", "value": "**example**"}},
            "textDocument/definition": location,
            "textDocument/references": [location],
            "textDocument/rename": {"changes": {uri: [edit]}},
            "textDocument/formatting": [edit],
            "textDocument/codeAction": [{"title": "Fix example", "kind": "quickfix", "edit": {"changes": {uri: [edit]}}}],
            "textDocument/documentSymbol": [{"name": "example", "kind": 12, "range": location["range"], "selectionRange": location["range"]}]
        }
        send({"id": request_id, "result": results[method]})

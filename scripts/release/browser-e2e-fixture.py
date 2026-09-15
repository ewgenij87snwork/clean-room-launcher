#!/usr/bin/env python3
import argparse
import json
import os
import re
import stat
import sys
from email.message import Message
from http.server import BaseHTTPRequestHandler, ThreadingHTTPServer
from pathlib import Path

MAX_BODY_BYTES = 1024 * 1024
EXPECTED_NAME = "CLROOM Canary"
EXPECTED_EMAIL = "clroom-canary@example.invalid"
SUCCESS_MARKER = "CLROOM_BROWSER_E2E_SUCCESS"


def fail(message: str) -> "NoReturn":
    raise SystemExit(message)


def expected_resume(path: Path) -> bytes:
    metadata = path.lstat()
    if stat.S_ISLNK(metadata.st_mode) or not stat.S_ISREG(metadata.st_mode):
        fail("synthetic resume must be a regular non-symlink file")
    if metadata.st_size > MAX_BODY_BYTES:
        fail("synthetic resume is unexpectedly large")
    return path.read_bytes()


def boundary_from(content_type: str) -> bytes | None:
    message = Message()
    message["content-type"] = content_type
    boundary = message.get_param("boundary", header="content-type")
    if not boundary or len(boundary) > 200:
        return None
    try:
        return boundary.encode("ascii")
    except UnicodeEncodeError:
        return None


def multipart_fields(body: bytes, boundary: bytes) -> dict[str, tuple[str | None, bytes]]:
    fields: dict[str, tuple[str | None, bytes]] = {}
    delimiter = b"--" + boundary
    for raw_part in body.split(delimiter)[1:]:
        if raw_part.startswith(b"--"):
            break
        if not raw_part.startswith(b"\r\n") or not raw_part.endswith(b"\r\n"):
            continue
        # Remove only the multipart framing CRLF. Payload bytes, including a
        # legitimate trailing newline in an uploaded file, must remain exact.
        part = raw_part[2:-2]
        header_blob, separator, payload = part.partition(b"\r\n\r\n")
        if not separator:
            continue
        headers = header_blob.decode("latin-1").split("\r\n")
        disposition = next(
            (line for line in headers if line.lower().startswith("content-disposition:")),
            "",
        )
        name_match = re.search(r'(?:^|;)\s*name="([^"]+)"', disposition)
        if not name_match:
            continue
        filename_match = re.search(r'(?:^|;)\s*filename="([^"]*)"', disposition)
        fields[name_match.group(1)] = (
            filename_match.group(1) if filename_match else None,
            payload,
        )
    return fields


def self_test() -> None:
    boundary = b"clroom-browser-self-test"
    resume = b"CLROOM SYNTHETIC RESUME\n"
    body = b"".join(
        [
            b"--" + boundary + b"\r\n",
            b'Content-Disposition: form-data; name="full_name"\r\n\r\n',
            EXPECTED_NAME.encode("utf-8") + b"\r\n",
            b"--" + boundary + b"\r\n",
            b'Content-Disposition: form-data; name="email"\r\n\r\n',
            EXPECTED_EMAIL.encode("utf-8") + b"\r\n",
            b"--" + boundary + b"\r\n",
            b'Content-Disposition: form-data; name="resume"; filename="resume.txt"\r\n',
            b"Content-Type: text/plain\r\n\r\n",
            resume + b"\r\n",
            b"--" + boundary + b"--\r\n",
        ]
    )
    fields = multipart_fields(body, boundary)
    expected = {
        "full_name": (None, EXPECTED_NAME.encode("utf-8")),
        "email": (None, EXPECTED_EMAIL.encode("utf-8")),
        "resume": ("resume.txt", resume),
    }
    if fields != expected:
        fail("browser fixture multipart self-test failed")
    if boundary_from('multipart/form-data; boundary="clroom-browser-self-test"') != boundary:
        fail("browser fixture boundary self-test failed")
    if multipart_fields(b"not-multipart", boundary):
        fail("browser fixture malformed-body self-test failed")
    print("BROWSER_E2E_FIXTURE_SELF_TEST_PASS")


def application_page() -> bytes:
    return f"""<!doctype html>
<html lang="en">
<head><meta charset="utf-8"><title>CLROOM Synthetic ATS</title></head>
<body>
  <h1>CLROOM Synthetic Browser Qualification</h1>
  <p>This page is local test data only. Do not enter real personal information.</p>
  <form action="/apply" method="post" enctype="multipart/form-data">
    <label>Full name <input name="full_name" required></label><br>
    <label>Email <input name="email" type="email" required></label><br>
    <label>Résumé <input name="resume" type="file" required></label><br>
    <button type="submit">Submit synthetic application</button>
  </form>
</body>
</html>
""".encode("utf-8")


def handler_class(result_path: Path, resume_bytes: bytes):
    class Handler(BaseHTTPRequestHandler):
        def log_message(self, _format: str, *_args) -> None:
            return

        def do_GET(self) -> None:
            if self.path != "/":
                self.send_error(404)
                return
            body = application_page()
            self.send_response(200)
            self.send_header("Content-Type", "text/html; charset=utf-8")
            self.send_header("Content-Length", str(len(body)))
            self.end_headers()
            self.wfile.write(body)

        def do_POST(self) -> None:
            if self.path != "/apply":
                self.send_error(404)
                return
            try:
                content_length = int(self.headers.get("Content-Length", ""))
            except ValueError:
                self.send_error(400)
                return
            if content_length <= 0 or content_length > MAX_BODY_BYTES:
                self.send_error(413)
                return
            boundary = boundary_from(self.headers.get("Content-Type", ""))
            if boundary is None:
                self.send_error(400)
                return
            body = self.rfile.read(content_length)
            fields = multipart_fields(body, boundary)
            valid = (
                fields.get("full_name", (None, b""))[1].decode("utf-8", "strict") == EXPECTED_NAME
                and fields.get("email", (None, b""))[1].decode("utf-8", "strict") == EXPECTED_EMAIL
                and fields.get("resume", (None, b""))[0] is not None
                and fields.get("resume", (None, b""))[1] == resume_bytes
            )
            if not valid:
                self.send_error(422)
                return
            record = {
                "schema": "clroom.browser-e2e-fixture.v1",
                "qualification": "PASS",
                "fields": ["email", "full_name", "resume"],
                "resume_bytes": len(resume_bytes),
            }
            result_path.write_text(
                json.dumps(record, sort_keys=True, separators=(",", ":")) + "\n",
                encoding="utf-8",
            )
            response = f"<html><body><h1>{SUCCESS_MARKER}</h1></body></html>".encode("utf-8")
            self.send_response(200)
            self.send_header("Content-Type", "text/html; charset=utf-8")
            self.send_header("Content-Length", str(len(response)))
            self.end_headers()
            self.wfile.write(response)

    return Handler


def main() -> None:
    if sys.argv[1:] == ["--self-test"]:
        self_test()
        return

    parser = argparse.ArgumentParser()
    parser.add_argument("--port-file", required=True, type=Path)
    parser.add_argument("--result-file", required=True, type=Path)
    parser.add_argument("--resume-file", required=True, type=Path)
    args = parser.parse_args()

    resume_bytes = expected_resume(args.resume_file)
    server = ThreadingHTTPServer(
        ("127.0.0.1", 0), handler_class(args.result_file, resume_bytes)
    )
    args.port_file.write_text(f"{server.server_port}\n", encoding="ascii")
    os.chmod(args.port_file, 0o600)
    try:
        server.serve_forever(poll_interval=0.1)
    finally:
        server.server_close()


if __name__ == "__main__":
    main()

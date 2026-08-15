import json
import os
import signal
import socket
import sqlite3
import subprocess
import time
import urllib.error
import urllib.request
from pathlib import Path

import httpx

REPO_ROOT = Path(__file__).resolve().parent.parent
DEFAULT_BINARY = REPO_ROOT / "target" / "debug" / "thy-squeal"

DEFAULT_SEED = [
    "CREATE TABLE items (id INTEGER PRIMARY KEY, name TEXT NOT NULL, price REAL)",
    "INSERT INTO items (name, price) VALUES ('widget', 9.99)",
    "INSERT INTO items (name, price) VALUES ('gadget', 3.50)",
]

DEFAULT_LONG_POLL = {
    "timeout_seconds": 2,
    "max_waiters": 1000,
    "max_waiters_per_client": 10,
}


def binary_path():
    override = os.environ.get("THYSQUEAL_BIN")
    if override:
        return Path(override)
    return DEFAULT_BINARY


def free_port():
    with socket.socket(socket.AF_INET, socket.SOCK_STREAM) as sock:
        sock.bind(("127.0.0.1", 0))
        return sock.getsockname()[1]


def yaml_value(value):
    if isinstance(value, bool):
        return "true" if value else "false"
    if isinstance(value, (int, float)):
        return repr(value)
    return json.dumps(str(value))


class ServerHarness:
    def __init__(
        self,
        tmp_path,
        *,
        databases=(("main", DEFAULT_SEED),),
        long_poll=None,
        cache=None,
    ):
        self.tmp = Path(tmp_path)
        self.tmp.mkdir(parents=True, exist_ok=True)
        self.host = "127.0.0.1"
        self.port = free_port()
        self.base_url = f"http://{self.host}:{self.port}"
        self.databases = list(databases)
        self.long_poll = {**DEFAULT_LONG_POLL, **(long_poll or {})}
        self.cache = dict(cache or {})
        self.process = None
        self._log_file = None
        self._client = None
        self._write_config()
        self._seed_databases()
        self._start()

    def _write_config(self):
        lines = [f'bind_address: "{self.host}:{self.port}"', "databases:"]
        for name, _seed in self.databases:
            lines.append(f"  - name: {yaml_value(name)}")
            lines.append(f"    path: {yaml_value(self.tmp / f'{name}.db')}")
        lines.append("long_poll:")
        for key, value in self.long_poll.items():
            lines.append(f"  {key}: {yaml_value(value)}")
        if self.cache:
            lines.append("cache:")
            for key, value in self.cache.items():
                lines.append(f"  {key}: {yaml_value(value)}")
        (self.tmp / "thy-squeal.yaml").write_text("\n".join(lines) + "\n")

    def _seed_databases(self):
        for name, statements in self.databases:
            if not statements:
                continue
            connection = sqlite3.connect(self.tmp / f"{name}.db")
            try:
                for statement in statements:
                    connection.execute(statement)
                connection.commit()
            finally:
                connection.close()

    def _start(self):
        self._log_file = open(self.tmp / "server.log", "wb")
        self.process = subprocess.Popen(
            [str(binary_path()), "--config", str(self.tmp / "thy-squeal.yaml")],
            stdout=self._log_file,
            stderr=subprocess.STDOUT,
            cwd=str(self.tmp),
        )
        self.wait_ready()

    def wait_ready(self, timeout=15.0):
        deadline = time.monotonic() + timeout
        while time.monotonic() < deadline:
            if self.process.poll() is not None:
                raise RuntimeError(
                    f"server exited early with code {self.process.returncode}:\n{self.log()}"
                )
            try:
                with urllib.request.urlopen(f"{self.base_url}/readyz", timeout=0.5) as response:
                    if response.status == 204:
                        return
            except (urllib.error.URLError, TimeoutError):
                pass
            time.sleep(0.05)
        raise TimeoutError(f"server did not become ready at {self.base_url}:\n{self.log()}")

    def log(self):
        try:
            return (self.tmp / "server.log").read_text()
        except OSError:
            return "<no server log>"

    @property
    def client(self):
        if self._client is None:
            self._client = self.new_client()
        return self._client

    def new_client(self):
        return httpx.Client(base_url=self.base_url, timeout=30)

    def get_json(self, path):
        return self.client.get(path)

    def post_json(self, path, payload):
        return self.client.post(path, json=payload)

    def diagnostics(self):
        return self.get_json("/api/diagnostics").json()

    def active_waiters(self):
        return self.diagnostics()["long_poll"]["active"]

    def stop(self):
        if self.process is None:
            return
        if self.process.poll() is None:
            self.process.terminate()
            try:
                self.process.wait(timeout=5)
            except subprocess.TimeoutExpired:
                self.process.kill()
                self.process.wait(timeout=5)
        self.process = None
        if self._client is not None:
            self._client.close()
            self._client = None
        if self._log_file is not None:
            self._log_file.close()
            self._log_file = None

    def signal_stop(self):
        if self.process is None or self.process.poll() is not None:
            return
        self.process.send_signal(signal.SIGINT)
        self.process.wait(timeout=15)


def wait_for(predicate, timeout=5.0, interval=0.05, message="condition not reached"):
    deadline = time.monotonic() + timeout
    while time.monotonic() < deadline:
        if predicate():
            return
        time.sleep(interval)
    raise AssertionError(message)


def raw_request(host, port, path, connection_close=False):
    sock = socket.create_connection((host, port), timeout=5)
    connection = "close" if connection_close else "keep-alive"
    sock.sendall(
        f"GET {path} HTTP/1.1\r\nHost: {host}:{port}\r\nConnection: {connection}\r\n\r\n".encode()
    )
    return sock


def read_all(sock, timeout=10.0):
    sock.settimeout(timeout)
    data = b""
    try:
        while True:
            chunk = sock.recv(4096)
            if not chunk:
                break
            data += chunk
    except socket.timeout:
        pass
    return data

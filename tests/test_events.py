import threading

from harness import raw_request, read_all, wait_for


def start_waiter(server, path="/api/events"):
    result = {}

    def worker():
        try:
            result["response"] = server.new_client().get(path, timeout=30)
        except Exception as error:  # pragma: no cover - surfaced via join checks
            result["error"] = error

    thread = threading.Thread(target=worker)
    thread.start()
    return thread, result


def test_event_delivery(server, client):
    thread, result = start_waiter(server, "/api/events?table=items&limit=1")
    wait_for(lambda: server.active_waiters() == 1)
    response = client.post(
        "/api/query",
        json={"sql": "INSERT INTO items (name, price) VALUES (?, ?)", "params": ["event", 1.5]},
    )
    assert response.status_code == 200
    thread.join(timeout=10)
    assert not thread.is_alive(), "long-poll waiter did not complete"
    assert "error" not in result
    waiter = result["response"]
    assert waiter.status_code == 200
    events = waiter.json()["events"]
    assert len(events) == 1
    assert events[0]["database"] == "main"
    assert events[0]["table"] == "items"
    assert events[0]["at"] > 0


def test_event_timeout(server, client):
    response = client.get("/api/events?limit=1", timeout=15)
    assert response.status_code == 408
    assert response.json()["error"]["code"] == "long_poll_timeout"


def test_event_validation(server, client):
    assert client.get("/api/events?limit=0").status_code == 400
    assert client.get("/api/events?limit=200").status_code == 400
    assert client.get("/api/events?table=").status_code == 400
    response = client.get("/api/events?db=nope")
    assert response.status_code == 400
    assert response.json()["error"]["code"] == "unknown_database"


def test_total_waiter_limit(harness):
    server = harness(long_poll={"timeout_seconds": 2, "max_waiters": 2, "max_waiters_per_client": 10})
    threads = [start_waiter(server) for _ in range(2)]
    wait_for(lambda: server.active_waiters() == 2)
    response = server.client.get("/api/events?limit=1", timeout=15)
    assert response.status_code == 503
    assert response.json()["error"]["code"] == "too_many_waiters"
    for thread, _ in threads:
        thread.join(timeout=15)
    assert not any(thread.is_alive() for thread, _ in threads)


def test_disconnect_releases_waiter(server):
    sock = raw_request(server.host, server.port, "/api/events")
    wait_for(lambda: server.active_waiters() == 1)
    sock.close()
    wait_for(lambda: server.active_waiters() == 0)


def test_shutdown_releases_waiters(harness):
    server = harness()
    sock = raw_request(server.host, server.port, "/api/events", connection_close=True)
    wait_for(lambda: server.active_waiters() == 1)
    server.signal_stop()
    data = read_all(sock, timeout=10)
    sock.close()
    assert b"shutting_down" in data, data
    assert server.process.poll() == 0

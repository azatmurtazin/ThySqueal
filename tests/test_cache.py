import pytest


def counters(server):
    return server.diagnostics()["databases"][0]["counters"]


def cache_state(server):
    return server.diagnostics()["databases"][0]


def test_cache_hit_and_miss_counters(server, client):
    query = {"sql": "SELECT id, name FROM items WHERE id = ?", "params": [1]}
    client.post("/api/query", json=query)
    client.post("/api/query", json=query)
    assert counters(server) == {
        "hits": 1,
        "misses": 1,
        "stores": 1,
        "invalidations": 0,
        "collection_runs": 0,
        "swept_entries": 0,
    }


def test_cache_key_includes_parameter_values(server, client):
    for param in (1, 2):
        client.post(
            "/api/query",
            json={"sql": "SELECT * FROM items WHERE id = ?", "params": [param]},
        )
    assert counters(server)["misses"] == 2
    assert counters(server)["stores"] == 2
    assert counters(server)["hits"] == 0


def test_cache_key_distinguishes_parameter_types(server, client):
    client.post("/api/query", json={"sql": "SELECT * FROM items WHERE id = ?", "params": [1]})
    client.post(
        "/api/query",
        json={"sql": "SELECT * FROM items WHERE id = ?", "params": ["1"]},
    )
    assert counters(server)["misses"] == 2
    assert counters(server)["stores"] == 2


def test_write_invalidates_cache(server, client):
    query = {"sql": "SELECT * FROM items WHERE id = ?", "params": [1]}
    client.post("/api/query", json=query)
    client.post("/api/query", json=query)
    response = client.post(
        "/api/query",
        json={"sql": "INSERT INTO items (name, price) VALUES (?, ?)", "params": ["x", 1.0]},
    )
    assert response.status_code == 200
    client.post("/api/query", json=query)
    snapshot = counters(server)
    assert snapshot["hits"] == 1
    assert snapshot["misses"] == 2
    assert snapshot["invalidations"] == 1


def test_nondeterministic_queries_bypass_cache(server, client):
    for _ in range(3):
        client.post("/api/query", json={"sql": "SELECT random()"})
    assert counters(server)["stores"] == 0
    assert counters(server)["hits"] == 0


def test_mark_and_sweep_reclaims_unused_entries(harness):
    seed = [
        "CREATE TABLE rows_t (id INTEGER PRIMARY KEY, label TEXT)",
        *[f"INSERT INTO rows_t (label) VALUES ('row-{i}')" for i in range(6)],
    ]
    server = harness(
        databases=(("main", seed),),
        cache={"max_entries": 10, "collection_threshold_entries": 2},
    )
    client = server.new_client()
    for param in range(1, 7):
        response = client.post(
            "/api/query",
            json={"sql": "SELECT id, label FROM rows_t WHERE id = ?", "params": [param]},
        )
        assert response.status_code == 200
    state = cache_state(server)
    assert state["cache_entries"] == 2
    assert state["counters"]["collection_runs"] == 4
    assert state["counters"]["swept_entries"] == 4

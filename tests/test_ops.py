def test_healthz(client):
    response = client.get("/healthz")
    assert response.status_code == 204


def test_readyz(client):
    response = client.get("/readyz")
    assert response.status_code == 204


def test_diagnostics_summary(server, client):
    client.post("/api/query", json={"sql": "SELECT 1"})
    body = server.diagnostics()
    assert isinstance(body["uptime_seconds"], (int, float))
    assert body["uptime_seconds"] >= 0
    assert body["long_poll"]["active"] == 0
    assert body["long_poll"]["max"] > 0
    assert body["long_poll"]["max_per_client"] > 0
    assert len(body["databases"]) == 1
    database = body["databases"][0]
    assert database["name"] == "main"
    assert database["cache_entries"] == 1
    assert database["cache_bytes"] > 0
    assert body["sqlite"]["executions"] >= 1


def test_diagnostics_html_dashboard(server, client):
    response = client.get("/diagnostics")
    assert response.status_code == 200
    assert response.headers["content-type"].startswith("text/html")
    assert "ThySqueal" in response.text
    assert 'id="databases"' in response.text

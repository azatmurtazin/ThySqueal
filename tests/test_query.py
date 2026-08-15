import pytest

from harness import ServerHarness

BOUNDARY_SEED = [
    "CREATE TABLE notes (id INTEGER PRIMARY KEY, body TEXT)",
    "INSERT INTO notes (id, body) VALUES (1, 'first')",
    "CREATE TABLE prices (id INTEGER PRIMARY KEY, amount REAL)",
]


def test_parameterized_read(client):
    response = client.post(
        "/api/query",
        json={"sql": "SELECT id, name, price FROM items WHERE id = ?", "params": [1]},
    )
    assert response.status_code == 200
    body = response.json()
    assert body["meta"]["columns"] == ["id", "name", "price"]
    assert body["meta"]["row_count"] == 1
    assert body["rows"] == [{"id": 1, "name": "widget", "price": 9.99}]


def test_write_metadata(client):
    response = client.post(
        "/api/query",
        json={"sql": "INSERT INTO items (name, price) VALUES (?, ?)", "params": ["new", 1.25]},
    )
    assert response.status_code == 200
    body = response.json()
    assert body["rows"] == []
    assert body["meta"]["rows_affected"] == 1
    assert body["meta"]["last_insert_id"] == 3


def test_update_metadata(client):
    response = client.post(
        "/api/query",
        json={"sql": "UPDATE items SET price = ? WHERE id = ?", "params": [2.5, 2]},
    )
    assert response.status_code == 200
    assert response.json()["meta"]["rows_affected"] == 1


def test_squeal_select(client):
    response = client.post(
        "/api/query",
        json={"squeal": {"_": "select", "from": "items", "cols": ["id", "name"]}},
    )
    assert response.status_code == 200
    body = response.json()
    assert body["meta"]["row_count"] == 2
    assert body["rows"][0]["name"] == "widget"


def test_squeal_with_params_rejected(client):
    response = client.post(
        "/api/query",
        json={"squeal": {"_": "select", "from": "items", "cols": ["*"]}, "params": [1]},
    )
    assert response.status_code == 400
    assert response.json()["error"]["code"] == "invalid_request"


def test_sql_and_squeal_rejected(client):
    response = client.post(
        "/api/query",
        json={
            "sql": "SELECT 1",
            "squeal": {"_": "select", "from": "items", "cols": ["*"]},
        },
    )
    assert response.status_code == 400
    assert response.json()["error"]["code"] == "invalid_request"


def test_neither_sql_nor_squeal_rejected(client):
    response = client.post("/api/query", json={"params": []})
    assert response.status_code == 400
    assert response.json()["error"]["code"] == "invalid_request"


def test_empty_sql_rejected(client):
    response = client.post("/api/query", json={"sql": "   "})
    assert response.status_code == 400
    assert response.json()["error"]["code"] == "invalid_request"


def test_invalid_json_rejected(client):
    response = client.post(
        "/api/query",
        content='{"sql":',
        headers={"content-type": "application/json"},
    )
    assert response.status_code == 400
    assert response.json()["error"]["code"] == "invalid_request"


def test_unknown_database(client):
    response = client.post("/api/query", json={"db": "nope", "sql": "SELECT 1"})
    assert response.status_code == 400
    assert response.json()["error"]["code"] == "unknown_database"


def test_policy_rejects_ddl(client):
    response = client.post("/api/query", json={"sql": "CREATE TABLE secret (id INTEGER)"})
    assert response.status_code == 422
    assert response.json()["error"]["code"] == "policy_rejection"


def test_policy_rejects_transaction(client):
    response = client.post("/api/query", json={"sql": "BEGIN"})
    assert response.status_code == 422
    assert response.json()["error"]["code"] == "policy_rejection"


def test_constraint_violation(client):
    response = client.post(
        "/api/query",
        json={"sql": "INSERT INTO items (id, name, price) VALUES (?, ?, ?)", "params": [1, "dup", 1.0]},
    )
    assert response.status_code == 400
    assert response.json()["error"]["code"] == "constraint_violation"


def test_invalid_sql(client):
    response = client.post("/api/query", json={"sql": "SELECT * FROM missing_table"})
    assert response.status_code == 400
    assert response.json()["error"]["code"] == "invalid_sql"


def test_malformed_squeal(client):
    response = client.post("/api/query", json={"squeal": {"_": "frobnicate"}})
    assert response.status_code == 400
    assert response.json()["error"]["code"] == "invalid_squeal"


@pytest.fixture
def boundary_server(harness):
    return harness(databases=(("main", BOUNDARY_SEED),))


@pytest.fixture
def boundary_client(boundary_server):
    client = boundary_server.new_client()
    yield client
    client.close()


def test_null_parameter(boundary_client):
    response = boundary_client.post(
        "/api/query",
        json={"sql": "INSERT INTO notes (id, body) VALUES (?, ?)", "params": [2, None]},
    )
    assert response.status_code == 200
    response = boundary_client.post("/api/query", json={"sql": "SELECT body FROM notes WHERE id = 2"})
    assert response.json()["rows"] == [{"body": None}]


def test_integer_boundaries(boundary_client):
    for value in (9223372036854775807, -9223372036854775808):
        response = boundary_client.post(
            "/api/query",
            json={"sql": "INSERT INTO notes (id, body) VALUES (?, ?)", "params": [value, "edge"]},
        )
        assert response.status_code == 200
    response = boundary_client.post(
        "/api/query",
        json={"sql": "SELECT id FROM notes WHERE id IN (?, ?)", "params": [9223372036854775807, -9223372036854775808]},
    )
    assert response.status_code == 200
    assert sorted(row["id"] for row in response.json()["rows"]) == [-9223372036854775808, 9223372036854775807]


def test_string_parameter_round_trip(boundary_client):
    texts = ["", "héllo wörld", 'quotes "and" \'apostrophes\'', "back\\slash/slash", "line\nbreak"]
    for index, text in enumerate(texts):
        response = boundary_client.post(
            "/api/query",
            json={"sql": "INSERT INTO notes (id, body) VALUES (?, ?)", "params": [10 + index, text]},
        )
        assert response.status_code == 200
    response = boundary_client.post(
        "/api/query",
        json={"sql": "SELECT body FROM notes WHERE id >= 10 ORDER BY id"},
    )
    assert [row["body"] for row in response.json()["rows"]] == texts


def test_float_parameter(boundary_client):
    response = boundary_client.post(
        "/api/query",
        json={"sql": "INSERT INTO prices (amount) VALUES (?)", "params": [3.25]},
    )
    assert response.status_code == 200
    response = boundary_client.post("/api/query", json={"sql": "SELECT amount FROM prices"})
    assert response.json()["rows"] == [{"amount": 3.25}]


def test_response_shape_is_stable(client):
    response = client.post("/api/query", json={"sql": "SELECT name FROM items ORDER BY id LIMIT 1"})
    body = response.json()
    assert set(body.keys()) == {"meta", "rows"}
    assert body["meta"]["columns"] == ["name"]
    assert body["meta"]["row_count"] == 1

# Long Polling

## Purpose

Long polling lets a client wait for a supported database-change event without
repeatedly issuing short-interval requests. It is intended for simple clients
that need timely change notification but do not require a persistent streaming
protocol.

## Endpoint

`GET /api/events` waits for database-change events. The connection stays open
until a matching event arrives, the configured timeout elapses, or the server
shuts down.

Query parameters are all optional:

| Parameter | Default | Description |
| --- | --- | --- |
| `db` | `main` | Name of the configured SQLite database to watch. |
| `table` | (any table) | Restrict responses to events for this table. A missing `table` matches events for any table, including events without a specific table. |
| `limit` | `1` | Maximum number of events to return, between `1` and `100`. |

Invalid filters, such as `limit` outside `1..=100` or an empty `table`, return
`400 invalid_request`. An unknown `db` returns `400 unknown_database`.

## Event Schema

Each event is published after a successful write that the SQL policy classifies
as a data change. Events are delivered through a Tokio `broadcast` channel with
a fixed capacity of 64; a waiter that misses messages and cannot catch up
silently resumes waiting for newer events. Events are not retained beyond the
channel capacity, so no unbounded history is kept.

```json
{
  "database": "main",
  "table": "items",
  "at": 1710000000000
}
```

- `database` is the name of the database the write committed to.
- `table` is the table the write targeted, when it can be determined from a
  single unambiguous target table; it is `null` otherwise. A write to an
  ambiguous scope still notifies waiters, matching any `table` filter.
- `at` is the server time of the write as Unix milliseconds.

## Responses

A success response returns up to `limit` matching events in the order they were
published, with `200 OK`:

```json
{
  "meta": { "database": "main" },
  "events": [ { "database": "main", "table": "items", "at": 1710000000000 } ]
}
```

If the wait timeout elapses before a matching event arrives, the server returns
`408` with code `long_poll_timeout`. A timeout that elapses after one or more
matching events were collected still returns those events with `200 OK`.

## Errors

| HTTP status | Error code | Condition |
| --- | --- | --- |
| `400` | `invalid_request` | Invalid `limit` or an empty `table`. |
| `400` | `unknown_database` | The `db` parameter names a database that is not configured. |
| `408` | `long_poll_timeout` | No matching event arrived within the configured timeout. |
| `429` | `too_many_waiters` | The client already holds its per-client waiter limit. |
| `503` | `too_many_waiters` | The total concurrent waiter limit is reached. |
| `503` | `shutting_down` | The server is shutting down. |

## Resource and Concurrency Controls

- Every request has a maximum wait duration configured by
  `long_poll.timeout_seconds`.
- The server limits concurrent waiters globally with
  `long_poll.max_waiters` and per client identity (the client's IP address and
  port) with `long_poll.max_waiters_per_client`.
- Dropping a request future — through timeout, disconnect, shutdown, or
  cancellation — unregisters its waiter promptly.
- Waiters and event payloads are bounded so idle or slow clients cannot leak
  memory or exhaust connections.

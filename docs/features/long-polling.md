# Long Polling

## Purpose

Long polling lets a client wait for a supported database-change event without
repeatedly issuing short-interval requests. It is intended for simple clients
that need timely change notification but do not require a persistent streaming
protocol.

## Request Lifecycle

The long-poll endpoint and event filter are defined as part of the public API.
A valid request registers a temporary waiter and keeps its HTTP connection open
until one of these outcomes occurs:

- A matching event is available; the server returns it successfully.
- The configured wait timeout elapses; the server returns a normal timeout
  response indicating that no event arrived.
- The client disconnects or cancels; the server removes the waiter.
- The server shuts down or cannot continue waiting; the request fails cleanly.

Events are published after successful relevant writes. They include enough
metadata for the client to identify the change, without requiring the server to
hold an unbounded event history.

## Resource and Concurrency Controls

- Every request has a maximum wait duration.
- The server limits concurrent waiters globally and, where applicable, per
  client identity.
- Disconnect detection promptly unregisters the waiter.
- Waiters and event payloads are bounded so idle or slow clients cannot leak
  memory or exhaust connections.

## Acceptance Criteria

- A waiting client receives a matching event after a successful write.
- A request with no matching event completes at the configured timeout.
- Client cancellation releases all server-side state.
- Invalid filters and excessive waiter counts return clear JSON errors.

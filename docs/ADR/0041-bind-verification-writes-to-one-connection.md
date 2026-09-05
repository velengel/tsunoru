# ADR 0041: Bind verification writes to one connection

## context

Review 3939865260 shows that an identity GET followed by a mutation on another connection can reach a replacement listener on the same port. Checking child liveness narrows that interval but cannot remove it. An established TCP connection does not become a new connection to a replacement listener.

## decision

Send each verification mutation over the same non-reconnecting TCP connection that returned its database identity.

## rejected options

- Add more liveness checks: separate checks retain a check-to-use interval.
- Enable ordinary keep-alive only: HTTP clients can reconnect automatically after closure.
- Add a new server guard: older concurrently running application binaries would not enforce it.

## consequences

Python disables reconnecting through a one-connect HTTPConnection subclass. The browser runner forwards mutations through a one-connect Node HTTP agent and fulfills the route with the actual response, preserving cookies and bytes. Closed identity connections fail before mutation instead of retrying. The server must support persistent HTTP/1.1 connections for verification writes; a closed connection causes a safe verification failure. Application code and its public API stay unchanged.

The connection hooks are documented by [Node HTTP Agent.createConnection](https://nodejs.org/api/http.html#agentcreateconnectionoptions-callback) and [Python HTTPConnection.connect](https://docs.python.org/3/library/http.client.html#http.client.HTTPConnection.connect). Local client source and deterministic listener-handover regressions verify the no-reconnect behavior used here.

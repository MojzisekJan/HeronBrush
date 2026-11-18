# Private live editor connections

The live MCP dialog copies `127.0.0.1:PORT#SECRET`. The random secret changes whenever the endpoint starts. Treat the complete invitation as a credential; do not publish it, commit it or include it in ordinary logs.

For a stdio MCP client, run `python3 /absolute/path/to/scripts/mcp_live_bridge.py` and set `HERON_MCP_ENDPOINT` in its environment to the copied invitation. The bridge sends the private admission line before forwarding JSON-RPC. Existing raw TCP clients must first send `HERON-MCP SECRET\n`; unauthenticated JSON-RPC is rejected. The headless `heron-brush --mcp` stdio protocol is unchanged.

The live endpoint accepts at most eight clients, eight pending requests and 16 MiB per input line including its newline. Authentication has a five-second read timeout, normal reads a 120-second timeout and socket writes five seconds. Idle clients may need to reconnect. Limits reject oversized input during reading, before JSON parsing.

LAN collaboration invitations now have the form `HOST:PORT#SECRET`. Copy the complete invitation into the join dialog. Desktop and web clients send the secret as their first session message; old clients without this support cannot join a protected LAN host. A failed invitation is never handed to the editor as an admitted session.

Browser origins are restricted to `https://mojzisekjan.github.io` and HTTP/HTTPS localhost origins. For another trusted deployment, set `HERON_COLLAB_ALLOWED_ORIGINS` on the desktop host to a comma-separated list of exact origins, such as `https://editor.example.org`. Do not add wildcards or untrusted websites. Native clients omit Origin but still require the invitation secret.

LAN TCP and plain WebSocket traffic is not encrypted. Authentication does not protect invitation secrets against network eavesdropping; use a trusted network or an encrypted transport/tunnel. The optional Iroh path has not been validated by these LAN changes.

Pending LAN admissions are bounded to eight, handshake workers to eight, session peer links to sixteen, and aggregate collaboration queues across the editor process to 512 MiB. Per-message/per-link bounds remain in force. The aggregate queue limit does not include all parser, geometry and renderer allocations.

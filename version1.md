'''


I'll create a single Rust file for rust-key-store (key-server and shard-router, run with --port 3000) and a separate kvs-client CLI, supporting one-off commands and REPL mode. The server uses axum for RESTful endpoints (POST, GET, PUT, DELETE) and clap for CLI parsing, with shard routing via a JSON config. The client uses reqwest for HTTP requests.
Plan:

key-server: RESTful key-value server with in-memory HashMap, supporting POST (201/409), GET (200/404), PUT (200/404), DELETE (200/404), and --help.

shard-router: Adds routing mode, forwarding requests based on key prefixes from a JSON config, acting as a reverse proxy.
client-cli: CLI with one-off commands (set, get, update, delete) and REPL mode, using clap and reqwest.

'''


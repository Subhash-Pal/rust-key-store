shard-router implementation.

✅ 1. Start Backend Key-Servers
powershell 2
3
4
5
# Terminal 1: key-server for "foo" prefix
cargo run -- --port 8081

# Terminal 2: key-server for "bar" prefix
cargo run -- --port 8082
These run in key-server mode (no --routes). 

2. Create routes.json
json
[
  { "prefix": "foo", "target": "http://127.0.0.1:8081" },
  { "prefix": "bar", "target": "http://127.0.0.1:8082" }
]
Save this as routes.json in your project root.

3. Start Shard Router
powershell
# Terminal 3
cargo run -- --port 3000 --routes routes.json
Output:
 Shard router listening on http://127.0.0.1:3000
4. Test with Inline JSON (PowerShell-safe)
powershell
1
# POST a value
curl.exe --% -X POST http://localhost:3000/keys/foo.x -H "Content-Type: application/json" -d "{\"val\": 42}"

# Expected response:
{"uri":"/keys/foo.x"}

# GET the value
curl.exe --% http://localhost:3000/keys/foo.x

# Expected response:
{"val":42}
This confirms routing: foo.x → backend 8081 with key .x

5. Test with File Input (Fixed Encoding)
powershell

# Create clean JSON file (ASCII = no BOM, no UTF-16)
'{"val": 42}' | Set-Content data.json -Encoding Ascii

# POST using file
curl.exe -X POST http://localhost:3000/keys/foo.y -H "Content-Type: application/json" -d "@data.json"

# Expected response:
{"uri":"/keys/foo.y"}

# Verify
curl.exe --% http://localhost:3000/keys/foo.y
# → {"val":42}
 6. Verify Backend Directly (Optional)
Check that backend key-server received the suffix key:

powershell
# Query backend directly (note the ".x" key)
curl.exe --% http://localhost:8081/keys/.x
# → {"val":42}
This proves the shard router stripped the prefix and forwarded correctly.

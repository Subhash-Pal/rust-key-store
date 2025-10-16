complete, step-by-step command sequence for two terminals that demonstrates your fully working Step 3: client-cli — with expected outputs.

🖥️ Terminal 1: Start the Key-Server
powershell

# Navigate to project root (if not already there)
cd D:\Rust Domain\Rust-Daily\rust-key-store

# Start the key-server on port 3000
cargo run --bin rust-key-store -- --port 3000
✅  Output:

 Key server listening on http://127.0.0.1:3000
💡 Leave this terminal running. 

🖥️ Terminal 2: Use the Client CLI
powershell



# Navigate to project root
cd D:\Rust Domain\Rust-Daily\rust-key-store

# 1. Set a key with JSON value
.\target\debug\kvs-client.exe --% set user.profile "{\"name\": \"Alice\", \"age\": 30}"

# 2. Get the value
.\target\debug\kvs-client.exe get user.profile

# 3. Update a key
.\target\debug\kvs-client.exe --% update user.profile "{\"name\": \"Alice\", \"age\": 31}"

# 4. Delete the key
.\target\debug\kvs-client.exe delete user.profile

# 5. Test REPL mode
.\target\debug\kvs-client.exe
> set user.repl {"active": true}
OK: /keys/user.repl
> get user.repl
{
  "active": true
}
> delete user.repl
OK: /keys/user.repl
> exit
✅ Expected Outputs:

text


# 1. Set
OK: /keys/user.profile

# 2. Get
{
  "name": "Alice",
  "age": 30
}

# 3. Update
OK: /keys/user.profile

# 4. Delete
OK: /keys/user.profile

# 5. REPL — as shown inline above
✅ Success Criteria Met
One-off commands
(
set
,
get
,
update
,
delete
)
✅
REPL mode with history

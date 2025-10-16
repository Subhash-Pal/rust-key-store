
🖥️ Terminal 1: Start the Key-Server
powershell


1
2
cd D:\Rust Domain\Rust-Daily\rust-key-store
cargo run --bin rust-key-store -- --port 3000

✅ output:



1
🔑 Key server listening on http://127.0.0.1:3000
🔸 Leave this running. 

🖥️ Terminal 2: Client CLI Commands (One-off + REPL)
powershell


cd D:\Rust Domain\Rust-Daily\rust-key-store

# 1. Set a key with nested JSON
.\target\debug\kvs-client.exe --% set user.profile "{\"name\": \"Alice\", \"age\": 30}"

# 2. Get the key
.\target\debug\kvs-client.exe get user.profile

# 3. Update the key
.\target\debug\kvs-client.exe --% update user.profile "{\"name\": \"Alice\", \"age\": 31}"

# 4. Delete the key
.\target\debug\kvs-client.exe delete user.profile

# 5. Test REPL mode
.\target\debug\kvs-client.exe
> set app.config {"debug": true}
OK: /keys/app.config
> get app.config
{
  "debug": true
}
> delete app.config
OK: /keys/app.config
> exit
✅ outputs:

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


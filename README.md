Output 

cargo run                                              
    Finished `dev` profile [unoptimized + debuginfo] target(s) in 0.97s
     Running `target\debug\rust-key-store.exe`
🔑 Key server running on http://127.0.0.1:3000


curl.exe --% -X POST http://localhost:3000/keys/test -H "Content-Type: application/json" -d "{\"x\":1}"
{"uri":"/keys/test"}
PS C:\Users\SUBHASH CHANDRA PAL> {"uri":"/keys/test"}

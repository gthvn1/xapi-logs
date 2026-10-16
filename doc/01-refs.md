# Refs in XAPI logs

In the task part of a log line, XAPI writes a short ref like `R:5fe71a7a2d59`.

```rust
use xapi_logs::refs::short_key;

assert_eq!(short_key("R:5fe71a7a2d59"), Some(0x5fe71a7a2d59));
```

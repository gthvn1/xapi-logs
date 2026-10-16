# Refs in XAPI logs

## What is a ref

Every object in the XAPI database (VM, SR, task, ...) has a ref. A ref is the
handle that XAPI uses to find the object. It looks like this:
```text
OpaqueRef:5fe71a7a-2d59-af9c-0ddb-89f1324b52d2
```

The part after `OpaqueRef:` is a random uuid, created by [`make`][make].

### Four kinds of ref

In XAPI, a ref has one of four kinds ([`type 'a t'`][type]):
```ocaml
type 'a t =
  | Real of string            (* ref to an object in the database *)
  | Dummy of string * string  (* ref to an object that is not in the database, with its name *)
  | Other of string           (* ref used for other purposes *)
  | Null
```

They are written like this:
```text
Real   OpaqueRef:5fe71a7a-2d59-af9c-0ddb-89f1324b52d2
Dummy  DummyRef:|c1baed25-a658-9de0-b212-200d2d2d9c9f|SR.scan
Other  any text
Null   OpaqueRef:NULL
```

A dummy ref is used for task that XAPI does not store in the database, for example
an internal call like `SR.scan`. It has a UUID and a name.

### Short refs in the task column

A full ref is long. In the task part of a log line, XAPI writes a short
version ([`really_pretty_and_small`][small]):
```text
[debug||831 /var/lib/xcp/xapi|VM.migrate_send R:5fe71a7a2d59|audit]
                                              ^^^^^^^^^^^^^^
```

- `R:` is a real ref. The task is in the XAPI database.
- `D:` is a dummy ref. The task is not in the XAPI database.
- `O:` is another kind of ref. XAPI keeps the first 12 characters of its text.

For `R:` and `D:`, XAPI keeps the 12 hex digits come from the uuid of the full
ref. We take the first 8 digits and the 4 digits after the first dash:

```text
OpaqueRef:5fe71a7a-2d59-af9c-0ddb-89f1324b52d2
          ^^^^^^^^ ^^^^
R:5fe71a7a2d59
```

In this project, [`short_key`][short_key] converts a short ref into a number. We use this
number as a key:
```rust
use xapi_logs::refs::short_key;

assert_eq!(short_key("R:5fe71a7a2d59"), Some(0x5fe71a7a2d59));
```

## Why it matters: one task, two log files

During a VM migration, XAPI scans a SR. In *xensource.log*, the scan is a dummy task
with a short ref:
```text
Mar 23 18:31:14 xcp-gtn-ip11 xapi: [debug||851 |VM.migrate_send R:5fe71a7a2d59|dummytaskhelper] task SR.scan D:c1baed25a658 created by task O:VM.migrate_s   
```

The scan itself is done by SM, which writes in *SMlog*. There, the same task appears
as a full dummy ref:
```text
'subtask_of': 'DummyRef:|c1baed25-a658-9de0-b212-200d2d2d9c9f|SR.scan'
```

Both give the same key, so we can follow the task from one file to the other.
[`key_of_ref`][key_of_ref] converts the full refs:
```rust
use xapi_logs::refs::{key_of_ref, short_key};

let in_smlog = key_of_ref("DummyRef:|c1baed25-a658-9de0-b212-200d2d2d9c9f|SR.scan");
let in_xensource = short_key("D:c1baed25a658");
assert_eq!(in_smlog, in_xensource);
```

## A trap: a ref is not a uuid

An object in the database has a ref **and** a `uuid` field. They are two different UUIDs.
In this line, the UUID is the `uuid` field of the task, not its ref:
```text
task Async.VDI.snapshot R:6cdc3a335e85 (uuid:8b98b2c1-9519-6bd8-c3b2-5dd61df04f3d) created
```

`8b98b2c1-9519` does not match `R:6cdc3a335e85`. Only the ref links a short ref to a full ref.


[make]: https://github.com/xapi-project/xen-api/blob/26.1-lcm/ocaml/xapi-types/ref.ml#L47
[type]: https://github.com/xapi-project/xen-api/blob/26.1-lcm/ocaml/xapi-types/ref.ml#L25
[small]: https://github.com/xapi-project/xen-api/blob/26.1-lcm/ocaml/xapi-types/ref.ml#L137
[short_key]: crate::refs::short_key
[key_of_ref]: crate::refs::key_of_ref

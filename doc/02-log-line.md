# The structure of a log line

XAPI does not write its log files itself. It sends each message to
syslog, and syslog writes the file (*xensource.log*, *SMlog*, ...).
So a line has two layers:

```text
Mar 23 18:31:11 xcp-gtn-ip11 xapi: [debug||831 /var/lib/xcp/xapi|VM.migrate_send R:5fe71a7a2d59|audit] ...
└───────── syslog ───────────────┘ └───────────── body, written by the daemon ───────────────────────────┘
```

## The Syslog layer

Syslog adds three things before the body: the time, the host and the daemon name.
```text
time host daemon: body
```

The time has two formats. By default, syslog writes it with 15 characters, with no year
and only seconds. A single-digit day is padded with a space:
```text
Mar  3 09:12:14
Mar 23 18:31:11
```

If syslog is set to high precision ([RFC 3339](https://www.rfc-editor.org/info/rfc3339/)),
the time has the year, the microseconds and the time zone:
```text
2026-03-23T18:31:11.123456+01:00
```

The daemon name is the program name. XAPI does not call `openlog` ([syslog.ml][openlog]),
so syslog uses the name of the program: `xapi`, `xenopsd-xc`, `xcp-networkd`, ...
In *SMlog*, it is `SM`.

## The body

All XAPI daemons are written in OCaml and use the same log library. The body is built by
[`format`][format] and sent to syslog by [`output_log`][output_log]. Two things happen
there:
- `output_log` calls `format false` so the body has no time of its own. The only time
  is the syslog time.
- It calls [`escape`][escape] on the body. New lines and non-ASCII characters are
  escaped, so one message is always one line.

SM is written in Python and uses another format:
```text
[181066][MainThread] lock: opening lock file /var/lock/sm/aa6f46c0-6b17-ac04-bdc2-0bc63589920b/sr
```

The body of a XAPI daemon is built with this [format string][sprintf]:
```ocaml
  Printf.sprintf "[%s%5s||%d %s|%s|%s] %s"
    ( if include_time then
        gettimestring ()
      else
        ""
    )
    priority id name task brand message
```

Let's look at each field.

### Time

The first `%s` is the time. It is always empty in syslog, because `output_log` calls
`format false`. So the body starts directly with the priority.

### Priority

There are four priorities: `debug`, `info`, `warn` and `error`. `%5s` pads them to
5 characters, thus `info` and `warn` start with a space:
```text
[debug||831 ...
[ info||673 ...
[ warn||526 ...
[error||272 ...
```

### The empty field

In XAPI 26.1, `||` is written as it is: this field is always empty. In newer versions,
it carries a [tracing context][remote_ctx].

### Thread

The thread is a number and a name, separated by a space. The name can be empty:
```text
831 /var/lib/xcp/xapi
4                          <- squeezed: no name
45 dbflush [/var/lib/xcp/state.db]
```

The name can contain spaces and `]`. When XAPI knows who sent the request, it adds
the client before the name, with `->`:
```text
272 HTTPS 10.1.0.100->:::80
    └── client ────┘  └name┘
```

### Task

The task is usually a name and a short ref (see [chapter 1](crate::chapter_01_refs)).
It can be empty, or a full ref:
```text
VM.migrate_send R:5fe71a7a2d59
session.logout D:0a4ee6f24878
OpaqueRef:84efd69f-050a-c857-6d69-ca645f4faf24
```

### Brand

The brand is the name of the OCaml module that wrote the line: `audit`, `xapi_session`, ...
It ends at the `]`.

### Message

The message is free text. It can contains `|` and `]`:
```text
[debug||831 ...|importexport] Piping HTTP { ... subtask_of = DummyRef:|d208e7e2-6ae6-0dbf-e3f0-1b75437f516e|VM.assert_can_migrate; ... }
```

It can also be empty. Then the line ends right after the `]`:
```text
[error||272 :::80|dispatch:event.from D:daaaf1d5ad77|backtrace]
```

## The grammar

This sums up the fields above. `[^|]*` means "any text without `|`".

```text
body     := '[' time priority '|' context '|' thread '|' task '|' brand ']' (' ' message)?
time     := ''                           (* always empty in syslog *)
priority := 'debug' | ' info' | ' warn' | 'error'
context  := [^|]*                        (* empty in 26.1 *)
thread   := id ' ' name
id       := [0-9]+
name     := [^|]*                        (* may contain spaces and ']' *)
task     := [^|]*
brand    := [^\]]*
message  := .*                           (* anything, including '|' and ']' *)
```

No field before the brand contains `|`, so the four `|` always split the same way.
The brand ends at the first `]` after the fourth `|`. Everything after it is the message.

## In this project

[`parse_line`][parse_line] splits the syslog layer from the body:
```rust
use xapi_logs::line::parse_line;

let line = parse_line(
    "Mar 23 18:31:11 xcp-gtn-ip11 xapi: [debug||831 /var/lib/xcp/xapi|VM.migrate_send R:5fe71a7a2d59|audit] VM.migrate_send"
).unwrap();
assert_eq!(line.host, "xcp-gtn-ip11");
assert_eq!(line.daemon, "xapi");
```


[openlog]: https://github.com/xapi-project/xen-api/blob/26.1-lcm/ocaml/libs/log/syslog.ml#L39
[format]: https://github.com/xapi-project/xen-api/blob/26.1-lcm/ocaml/libs/log/debug.ml#L90-L111
[output_log]: https://github.com/xapi-project/xen-api/blob/26.1-lcm/ocaml/libs/log/debug.ml#L145-L150
[escape]: https://github.com/xapi-project/xen-api/blob/26.1-lcm/ocaml/libs/log/debug.ml#L88
[sprintf]: https://github.com/xapi-project/xen-api/blob/26.1-lcm/ocaml/libs/log/debug.ml#L105-L111
[remote_ctx]: https://github.com/xapi-project/xen-api/blob/40dcca9ea43b316bd2277fbdd6da92666dd19419/ocaml/libs/log/debug.ml#L113-L117
[parse_line]: crate::line::parse_line

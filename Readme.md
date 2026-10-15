# xapi-logs

The project is just starting. There is no tool to run yet, but the documentation
already explains some XAPI concepts found in the logs.

## Read the documentation

The documentation is online: https://gthvn1.github.io/xapi-logs/
It is updated each time a change is pushed on `master`.

You can also build it locally. You need Rust 1.85 or newer. You can install it with
[rustup](https://rustup.rs), or use the Nix development shell: `nix develop`
(or let direnv load it). Then build the documentation and open it in your
browser: `cargo doc --open`

Start with the `refs` module. It explains:
- what the short refs like `R:5fe71a7a2d59` and `D:c1baed25a658` are,
- how a short ref is built from a full ref like `OpaqueRef:<uuid>`,
- how we link them with a key.

Each function has examples. They are also tests.

## Run the tests

- `cargo test`

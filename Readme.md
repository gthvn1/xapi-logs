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


## Notes

### Steel-poc

- It is really long to compile and run it in debug mode. To improve things we do:
    - Steel itself is compiled without optimization (execution is 10x faster): `opt-level = 3` for dependencies, our code is still in debug.
    - Dwarf was 112MB, inflated by optimization. It took a long time to link: `debug = "line-tables-only"`
    - By defaut nix provides an olf GNU ld (BFD) linker: `mold via useMoldLinker` in flake.nix

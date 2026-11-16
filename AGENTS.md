# papercuts

A Rust CLI for recording and retrieving papercut incident reports. README.md
describes the behavior; `skills/papercuts/SKILL.md` is the agent-facing guide
and must stay in sync with the CLI's flags.

Before finishing a change, run what CI runs:

```sh
cargo fmt --check
cargo clippy --all-targets --locked -- -D warnings
cargo test --locked
```

`rust-toolchain.toml` pins the toolchain. Use rustup's `cargo` so it applies;
some version-manager shims ignore it.

Pin dependency versions exactly (`=x.y.z`) and commit `Cargo.lock`.

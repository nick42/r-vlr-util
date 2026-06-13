# r-vlr-util

General-purpose Rust utilities inspired by the C++ `vlr-util` library.

This is a learning-oriented Rust port, not a line-for-line translation. It
prefers Rust standard-library types and established Rust idioms, and adds
modules as useful behavior is migrated.

## Getting started

```powershell
cargo test
cargo fmt --check
cargo clippy --all-targets --all-features -- -D warnings
cargo run --example split_path
cargo doc --open
```

## Layout

- `src/lib.rs`: the public library root
- `src/*.rs`: library modules, introduced as behavior is ported
- `tests/`: integration tests exercising the public API
- `examples/`: small runnable programs showing library usage
- `docs/`: design and migration notes

See [docs/PORTING.md](docs/PORTING.md) for the initial porting strategy.

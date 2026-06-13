# r-vlr-util

General-purpose Rust utilities inspired by the C++ `vlr-util` library.

This is an idiomatic behavioral Rust port, not a line-for-line translation.
It includes the portable utility surface, isolated Win32 modules, and a stable
C ABI with C++ convenience adapters.

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
See [MIGRATION.md](MIGRATION.md) for a detailed C++-to-Rust migration guide
and compatibility notes.

## Modules

- `strings`, `text`: splitting, comparison, and similarity
- `numeric`, `display`, `data`: checked conversions and data adaptors
- `cache`, `retry`, `scope`: reusable utility patterns
- `options`: typed application options and basic configuration files
- `filesystem`, `network`: portable platform abstractions
- `logging`, `threading`, `shared_registry`: application infrastructure
- `windows`: owned Win32 resources, registry, filesystem, COM, services,
  security, dynamic loading, and runtime helpers
- `ffi`: stable C ABI used by headers under `include/`

## C++ consumers

Build with `cargo build --release`, add `include/` to the C++ include path,
and link `target/release/r_vlr_util.dll.lib`. See
[CXX_COMPATIBILITY.md](CXX_COMPATIBILITY.md) for the supported adapter model
and the C++ patterns that cannot be reproduced by a Rust binary.

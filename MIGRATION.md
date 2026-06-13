# vlr-util C++ to Rust migration guide

This document explains how the material behavior in the C++ `vlr-util`
library maps into `r-vlr-util`. The Rust project is a behavioral migration,
not a line-for-line translation. C++ utilities that exist mainly to compensate
for C++ language or platform constraints are replaced with native Rust
features instead of being reproduced.

## High-level approach

The original project is a reusable collection of utilities, templates,
platform adaptors, and experiments. The Rust version remains one library
crate, but groups behavior into domain-oriented modules under `src/`.

The migration followed these rules:

1. Preserve useful externally observable behavior.
2. Prefer Rust standard-library types and ownership patterns.
3. Use `Result<T, E>` and `Option<T>` instead of success/failure result codes.
4. Use borrowing where returned values can safely refer to input data.
5. Keep platform-neutral functionality cross-platform.
6. Do not recreate C++ compile-time dispatch, compatibility shims, or build
   macros that have no Rust equivalent.
7. Use unit tests beside implementation and integration tests for public API
   behavior.

## Project organization

| Rust module | Material C++ sources represented |
| --- | --- |
| `strings` | `strings.split.h` and string-view slicing helpers |
| `conversion` | useful UTF-8/UTF-16 and C-string boundaries from string conversion helpers |
| `text` | `StringCompare`, `CloseEnough`, `LevenshteinDistance` |
| `numeric` | bit helpers, `MathOps`, `range_checked_cast`, CRC32, resource lookup value |
| `data` | `util.data_adaptor.MultiSZ` |
| `cache` | `MRUCache`, `RegexCache` |
| `retry` | `RetrySettings`, `CallWithAutoRetry` intent |
| `scope` | `ActionOnDestruction`, `AutoFreeResource`, `AutoRevertingAssignment` |
| `filesystem` | portable `platform.FileSystemOps` behavior |
| `options` | app-option values, matching, qualifiers, metadata, and basic file source |
| `logging` | code/message context, levels, callback-style logging |
| `threading` | thread pool and thread operation context |
| `shared_registry` | shared-instance registrar |
| `network` | network target information |
| `display` | approximate data-size formatting |
| `windows::*` | owned Win32 handles, registry, filesystem, COM, services, security, dynamic loading, and runtime helpers |
| `ffi` and `include/` | stable C ABI and C++ compatibility adapters |

## Core language-pattern translations

### Ownership replaces manual lifetime management

C++ uses `shared_ptr`, `weak_ptr`, custom cleanup classes, and explicit
destructors throughout the original project. Rust values have one owner by
default and are destroyed automatically when that owner leaves scope.

- `CActionOnDestruction` becomes `scope::ScopeGuard`.
- `CAutoFreeResource` usually needs no custom type. A resource-owning Rust
  type implements `Drop`; a one-off cleanup can use `ScopeGuard`.
- `CAutoRevertingAssignment` becomes `scope::RevertGuard`.
- `CNonOwningSharedPtr` is not ported. Rust expresses non-ownership with
  references such as `&T`, and shared ownership with `Arc<T>`.
- `CBaseWithVirtualDestructor` is unnecessary. Rust trait objects always run
  the concrete value's destructor.

The Rust borrow checker also prevents a variable from being independently
accessed while `RevertGuard` holds its mutable reference. Access goes through
the guard. This differs from C++ and removes an aliasing hazard.

### Views, spans, and null-terminated strings

- `std::string_view` maps to `&str`.
- `as_span<T>` maps to `&[T]` or a generic `AsRef<[T]>` parameter.
- `zstring_view` and `logical_zstring_view` are not reimplemented as general
  string types. Rust strings are not implicitly C strings. FFI boundaries
  should use `std::ffi::CStr` and `CString`, making the null-termination
  requirement explicit.
- Returned split elements borrow from the input as `&str`; they do not allocate
  individual strings.

Rust `str` is valid UTF-8. The C++ library supports narrow and wide character
types independently. Rust instead uses UTF-8 internally and converts at FFI
or operating-system boundaries.

### Errors and result codes

`SResult`, HRESULT conversion helpers, `MakeResultCode`, and result-checking
macros are replaced by standard Rust results:

```rust
fn operation() -> Result<Value, Error> {
    let intermediate = fallible_step()?;
    Ok(make_value(intermediate))
}
```

`?` replaces the family of `VLR_ON_ERROR...` and
`VLR_ASSERT...OR_RETURN...` macros. `Option<T>` represents a missing value
without manufacturing a special success code such as `S_FALSE`.

### Templates and overload dispatch

Several C++ utilities exist to select overloads or emulate algebraic data
types:

- `util.choice` is unnecessary because Rust traits and explicit enums provide
  deterministic dispatch.
- `util.overloaded` and `std::variant` visitors become `match` expressions.
- Smart-enum formatting/range checking becomes an ordinary Rust `enum` with
  `Display`/`TryFrom` implementations when needed.
- Sequential enum range iteration becomes a normal iterator or explicit
  constant array. Rust deliberately does not assume every integer between two
  discriminants is a valid enum.

### Thread safety

Rust requires shared cross-thread data to satisfy `Send` and `Sync`.

- The regex cache uses `RwLock<HashMap<...>>` and returns `Arc<Regex>`.
- The shared registry stores typed `Arc<T>` values and rejects a lookup using
  the wrong type.
- Operation context is truly thread-local via `thread_local!`; a context guard
  must be dropped on the thread that created it.
- The thread pool uses standard channels and owns its worker threads. Dropping
  the pool sends shutdown messages and joins every worker.

## Behavioral decisions by subsystem

### String comparison and similarity

`text::Comparator` preserves case-sensitive and case-insensitive comparison,
prefix, suffix, and substring operations. Case-insensitive behavior is
explicitly ASCII case-insensitive, matching the practical behavior of the
original C-runtime-oriented implementation without claiming full Unicode
case-folding.

Levenshtein distance operates on Unicode scalar values rather than bytes. This
is generally more useful for Rust UTF-8 strings and means a non-ASCII character
counts as one character edit.

The original `CloseEnough` implementation currently falls back to Levenshtein
distance. Rust exposes that useful behavior directly through
`closest_candidates`.

### Numeric conversions

C++ casts can silently truncate, wrap, or change signedness. The Rust
`numeric::checked_cast` helper is based on `TryFrom` and returns an error when
the value does not fit. App-option conversion follows the original accepted
types but reports invalid conversions rather than saturating after an
assertion.

Floating-point-to-integer option conversion preserves the C++ truncation
toward zero when the finite value is in range.

### Caches

`MruCache` fixes a behavioral weakness in the C++ implementation: updating or
reading a key updates its recency without leaving duplicate keys in the order
list. Inserting beyond capacity evicts the least recently used entry.

`RegexCache` uses the well-established Rust `regex` crate. Rust regex syntax
does not support backreferences or look-around, but it guarantees linear-time
searching and avoids catastrophic backtracking.

### MULTI_SZ

`data::parse_multi_sz` and `encode_multi_sz` preserve the double-NUL-terminated
layout while accepting generic element types such as `u8` or `u16`.
Malformed buffers and embedded empty values return `MultiSzError`.

### Application options

The option subsystem preserves:

- native-name storage;
- case-insensitive hierarchical name matching, with `:` and `.` equivalent;
- typed values and conversions;
- qualifiers and custom metadata;
- basic `name = value` file ingestion;
- comments, blank lines, quoted values, and trailing comments.

`OptionDefinition<T>` represents the typed access pattern from
`AppOptionAccess`: it converts a specified value when present and otherwise
returns the application's typed default.

The C++ subsystem caches conversions on first access. The Rust implementation
does not currently cache converted forms because conversion is explicit and
cheap for the supported values. This avoids hidden interior mutation and
keeps references simple.

The `ReturnOnlyDefaultValue` qualifier causes `AppOptions::get` to return
`None`. Default values belong at the consuming application's typed option
definition, rather than inside this untyped store.

### Filesystem and platform behavior

Portable file existence, directory existence, deletion, and temporary
directory discovery use `std::fs`, `Path`, and `PathBuf`.

Win32-only behavior is isolated under `src/windows/` and compiled only on
Windows. It includes owned handles, registry access, file/volume enumeration,
code-page conversion, GUID/COM helpers, service control, SID/token operations,
ACL/ACE snapshots, security descriptors, `FILETIME`, dynamic loading,
platform/runtime queries, and console capture.

The portable core contains no unsafe blocks. The crate warns on unsafe code,
and `windows`/`ffi` explicitly allow it only at reviewed FFI boundaries. Raw
ACLs and SIDs are copied into owned Rust values before ordinary callers inspect
them. Service and registry handles close automatically through `Drop`.

`RegistryKey::read_values_as_options` replaces `CAppOptionSource_Registry`.
`CreateServiceConfig` preserves the original validation against invalid names
and unquoted executable paths containing spaces.

### C ABI and C++ consumption

The crate emits `rlib`, `staticlib`, and `cdylib` artifacts. `src/ffi.rs`
exports a versioned C ABI, while `include/vlr-util/compat.hpp` supplies C++
adapters for representative familiar APIs. ABI booleans use `uint8_t`, GUIDs
have explicit C layout, and conversions use caller-owned buffers.

A real MSVC consumer in `tests/cpp_consumer.cpp` verifies linking and execution
against the Rust DLL. See `CXX_COMPATIBILITY.md` for the exact compatibility
boundary and the definitive reasons a literal C++ source/binary drop-in is
impossible.

## Components replaced by Rust itself

| C++ component | Rust replacement |
| --- | --- |
| assertion-return macros | `assert!` for invariants; `Result` plus `?` for recoverable errors |
| `formatpf` / `fmt` adaptors | `format!`, `write!`, formatting traits |
| string conversion overload matrix | UTF-8 `String`/`&str`; explicit OS/FFI conversion |
| `as_string_view`, `as_span` | `&str`, `&[T]`, `AsRef` |
| `zstring_view` | `CStr`/`CString` at FFI boundaries |
| `NonOwningSharedPtr` | references; `Arc` only for ownership |
| `OnValidAssignTo` | parsing/conversion returning `Result` |
| `IsNonZero`, `IsNotBlank` | direct comparisons, `is_empty`, or domain-specific predicates |
| `util.choice`, `util.overloaded` | traits, enums, `match` |
| precompiled headers, SAL, target-version headers | Cargo/rustc; explicit types and safety rules |
| CMake/vcpkg project glue | Cargo manifest and Cargo.lock |

## Compatibility expectations

- Material portable and Win32 behavior is represented and tested through the
  native Rust API.
- C and C++ can consume the stable exported C ABI; selected C++ adapters
  preserve familiar names and RAII-style use.
- Exact C++ source and binary compatibility is impossible for templates,
  STL/ATL types, virtual classes, macros, and compiler-specific ABI. These
  definitive gaps are catalogued in `CXX_COMPATIBILITY.md`.
- Text uses UTF-8 rather than `TCHAR`/wide-string build modes.
- Case-insensitive comparison is ASCII-oriented.
- Error behavior is explicit through `Result`/`Option`, so callers must handle
  failures instead of inspecting HRESULT-like values.

## Reading the migration

A useful order for a C++ developer learning the Rust version is:

1. `src/strings.rs` for borrowing and returned string slices.
2. `src/scope.rs` for ownership and `Drop`.
3. `src/numeric.rs` and `src/options.rs` for `Result`, enums, and `match`.
4. `src/cache.rs` and `src/shared_registry.rs` for `Arc`, `Mutex`, and `RwLock`.
5. `src/threading.rs` for channels, owned threads, and thread-local state.
6. `src/windows/handle.rs`, then the other `src/windows/` modules, for scoped
   unsafe FFI and RAII ownership.
7. `src/ffi.rs` and `include/vlr-util/compat.hpp` for the C++ boundary.

Run the complete quality suite with:

```powershell
cargo fmt --check
cargo test --all-features
cargo clippy --all-targets --all-features -- -D warnings
cargo doc --no-deps
```

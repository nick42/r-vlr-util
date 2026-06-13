# Porting strategy

The goal is to preserve useful behavior from `vlr-util`, not its C++-specific
implementation or organization.

## Starting principles

1. Look for an existing Rust standard-library feature or mature crate before
   porting a utility.
2. Port one cohesive behavior at a time, together with its tests.
3. Prefer borrowed values such as `&str` and slices when ownership is not
   needed.
4. Represent recoverable failures with `Result` and optional values with
   `Option`.
5. Isolate Windows-only behavior with `#[cfg(windows)]` when it is introduced.
6. Introduce additional crates or a Cargo workspace only when real boundaries
   emerge.

## Initial mapping

| C++ concept | Rust direction |
| --- | --- |
| `strings.split.h` | `src/strings.rs`; first behavior ported |
| `std::string_view` / `zstring_view` | Usually `&str`, `CStr`, or `CString` |
| `as_span` | Usually `&[T]` or `AsRef<[T]>` |
| `SResult` / `util.Result` | `Result<T, E>` |
| `util.choice` | Enums and pattern matching |
| `ActionOnDestruction` | Ownership and `Drop` |
| `AutoRevertingAssignment` | A scoped guard type, only if still needed |
| `ThreadPool` | Evaluate a mature crate before porting |
| Win32 utilities | A future `windows` module behind `cfg(windows)` |

## Why this is one crate

The C++ project is one reusable library. A single Rust library crate is the
simplest corresponding shape and keeps navigation and builds easy while the
API is still taking form. Cargo workspaces are valuable once there are genuine
independently versioned or independently consumed packages; adding one now
would create ceremony without a boundary.

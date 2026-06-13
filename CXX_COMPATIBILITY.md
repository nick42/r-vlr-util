# C++ compatibility and unavoidable gaps

`r-vlr-util` builds as a Rust library, a static library, and a DLL. C and C++
programs can consume the stable C ABI declared in
`include/r-vlr-util/r-vlr-util.h`; `include/vlr-util/compat.hpp` adds small
RAII-style C++ adapters using familiar `vlr` names.

The included `tests/cpp_consumer.cpp` is compiled with MSVC and linked against
the Rust DLL during compatibility verification.

## What "drop-in" can mean

The Rust DLL can replace implementations behind a stable C ABI. It cannot be
a literal source-compatible or binary-compatible replacement for the original
C++ library. Existing C++ projects must change their include/link setup and,
for APIs not covered by an adapter, use a purpose-built C ABI wrapper or call
the native Rust API from a Rust component.

This limitation is definitive, not merely unfinished work:

| Original C++ pattern | Why a Rust library cannot reproduce it exactly |
| --- | --- |
| Header-only templates, overload sets, concepts, and specialization | These are instantiated by the C++ compiler in the consumer; a Rust binary cannot export them. |
| `std::string`, `std::vector`, `std::function`, `std::shared_ptr`, RTTI, and exceptions | Their ABI depends on C++ compiler/runtime/version and is not Rust's ABI. |
| ATL/MFC `CString`, BSTR helper overloads, `TCHAR` build modes | ATL/MFC types and narrow/wide build selection belong to the C++ compilation environment. |
| Inline classes, inherited Win32 structures, virtual interfaces, and vtables | Rust cannot promise MSVC class layout, name mangling, RTTI, or vtable compatibility. |
| Macros, SAL annotations, precompiled headers, and target-version headers | They alter or annotate C++ compilation and have no runtime symbol to replace. |
| Arbitrary borrowed pointers/views and non-owning `shared_ptr` tricks | A stable ABI cannot express or enforce their C++ lifetime and aliasing contracts. |
| Generic typed dynamic-function casts | Converting a raw symbol to an arbitrary function signature is necessarily caller-specific and unsafe. |

## Compatibility levels

- **Native Rust behavior:** all material portable and Win32 behavior has an
  idiomatic Rust API and tests.
- **Stable ABI behavior:** representative high-value operations are exported
  through `extern "C"` using fixed-layout integers, pointers, lengths, and a
  two-call output-buffer protocol.
- **C++ convenience behavior:** `compat.hpp` currently adapts `SResult`,
  CRC32, UTF conversion, debugger detection, and GUID creation.
- **Impossible exact compatibility:** the C++ patterns listed above require
  source changes or C++-side adapters.

The C ABI deliberately does not expose Rust-owned `String`, `Vec`, trait
objects, closures, panics, or allocator-owned memory. This keeps ownership and
compiler-runtime boundaries explicit.

## Building and linking from C++

```powershell
cargo build --release
```

On Windows this produces `r_vlr_util.dll`, `r_vlr_util.dll.lib`, and
`r_vlr_util.lib` under `target/release/`. Add `include/` to the C++ include
path, link the import library, and deploy the DLL beside the executable.

## Extending the adapter

New cross-language operations should first be implemented and tested as
native Rust. Add a narrow C ABI function in `src/ffi.rs`, using caller-owned
buffers or opaque handles, then add an inline C++ adapter. Never pass
Rust-owned containers, C++ standard-library objects, exceptions, or callbacks
with undocumented lifetimes across the ABI.

//! The `OpenBim.Ifc` NuGet package: .NET bindings for `openbim-ifc` (#327).
//!
//! The binding is C#, in `dotnet/OpenBim.Ifc/`: P/Invoke over the versioned
//! C ABI of `openbim-ifc-capi` (`openbim_ifc_v0_1_*`), whose shared library
//! the package ships per runtime identifier under `runtimes/<rid>/native/`.
//! It adds calling-convention glue only (ADR 0013): every refusal and every
//! error code comes from the same Rust core the other bindings use.
//!
//! This crate holds no Rust binding code. It exists so the package has what
//! every other binding has -- a version in a manifest the release tooling
//! reads, a changelog, a `openbim-ifc-dotnet-v*` release tag and a reference
//! page -- and so its tests can hold the C# source to the two contracts it
//! depends on without a .NET SDK:
//!
//! - every `openbim_ifc_v0_1_*` function and status code of the C header is
//!   declared in `Native/NativeMethods.cs` with the header's parameter
//!   types, and every struct with its fields in order;
//! - every domain record of the shared binding core has a C# record with
//!   the same fields in the same order, since a record crosses the C ABI as
//!   a `LIST` of its field values with no names.
//!
//! The behaviour is tested from C# against the packed `.nupkg` by
//! `scripts/check-dotnet.py`.

/// The NuGet package id.
pub const PACKAGE_ID: &str = "OpenBim.Ifc";

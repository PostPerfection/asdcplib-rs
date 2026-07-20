# Design

Safe Rust bindings for asdcplib (SMPTE AS-DCP / AS-02 MXF).

## Crates

- `asdcplib-sys`: raw FFI. C shim (`shim/asdcp_shim.cpp`) wraps ASDCP::/AS_02:: classes in extern "C" functions. `build.rs` builds vendored asdcplib via CMake and links it statically.
- `asdcplib`: safe wrappers over the sys crate.

## Coverage

- AS-DCP writers/readers: JP2K (mono + stereo), PCM, timed text, Atmos.
- AS-02 (IMF) writers/readers: JP2K, PCM, timed text.
- Crypto contexts (AES encryption, HMAC) plumbed through writer/reader options, with an encrypted JP2K roundtrip test.
- `essence_type` probe and library version.

## Testing

36 tests + 1 doctest, byte-exact MXF roundtrips through the real C++ library for all six reader/writer pairs, plus an encrypted (AES + HMAC) JP2K roundtrip.

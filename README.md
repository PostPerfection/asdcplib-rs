# asdcplib-rs

[![CI](https://github.com/PostPerfection/asdcplib-rs/actions/workflows/ci.yml/badge.svg)](https://github.com/PostPerfection/asdcplib-rs/actions/workflows/ci.yml)

[Documentation](https://postperfection.github.io/asdcplib-rs/)

Rust FFI bindings for [asdcplib](https://github.com/cinecert/asdcplib), the MXF file access library used in Digital Cinema. These bindings cover AS-DCP (ST 429) and AS-02/IMF (ST 2067-5) read/write.

## Crates

| Crate | Description |
|---|---|
| `asdcplib-sys` | Raw FFI bindings with C shim, builds asdcplib from source via CMake |
| `asdcplib` | Safe Rust wrapper with typed readers/writers for AS-DCP and AS-02 JP2K, PCM, TimedText, plus AS-DCP Atmos |

## Prerequisites

- CMake 3.10+
- C++14 compiler (GCC, Clang)
- OpenSSL development headers (`libssl-dev`)
- Optional: Xerces-C (`libxerces-c-dev`) for Timed Text support

## Usage

```toml
[dependencies]
asdcplib = { git = "https://github.com/PostPerfection/asdcplib-rs.git", tag = "v0.1.0" }
```

```rust
use asdcplib::{essence_type, EssenceType};
use asdcplib::jp2k;

// Detect essence type
let etype = essence_type("picture.mxf")?;

// Read JP2K frames
let mut reader = jp2k::MxfReader::new();
reader.open_read("picture.mxf")?;
let desc = reader.picture_descriptor()?;
let mut buf = vec![0u8; 10 * 1024 * 1024]; // 10MB buffer
let size = reader.read_frame(0, &mut buf, None, None)?;
```

## Supported Essence Types

AS-DCP (d-cinema, ST 429) read/write:

- JPEG 2000 (mono + stereoscopic 3D)
- PCM audio (24-bit, 48kHz / 96kHz)
- Timed Text (SMPTE ST 429-5)
- Dolby Atmos (IAB)

AS-02 (IMF, ST 2067-5) read/write, in the `as02` module:

- JPEG 2000 (frame-wrapped)
- PCM audio (clip-wrapped)
- Timed Text (SMPTE ST 2067-2)

Other AS-02 essence (ISXD, ACES, IAB, JPEG XS) is detection-only via `essence_type`.

## Building

The `asdcplib-sys` crate expects the asdcplib source tree at `asdcplib-sys/asdcplib/`:

```bash
git submodule update --init --recursive
cargo build
cargo test
```

The integration suite writes and reads a real PCM MXF to verify the safe wrapper and C++ library together.

## License

BSD-3-Clause (matches asdcplib upstream)

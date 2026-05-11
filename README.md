# asdcplib-rs

Rust FFI bindings for [asdcplib](https://github.com/cinecert/asdcplib) — the AS-DCP and AS-02 MXF file access library used in Digital Cinema.

## Crates

| Crate | Description |
|---|---|
| `asdcplib-sys` | Raw FFI bindings with C shim, builds asdcplib from source via CMake |
| `asdcplib` | Safe Rust wrapper with typed readers/writers for JP2K, PCM, TimedText, Atmos |

## Prerequisites

- CMake 3.10+
- C++14 compiler (GCC, Clang)
- OpenSSL development headers (`libssl-dev`)
- Optional: Xerces-C (`libxerces-c-dev`) for Timed Text support

## Usage

```toml
[dependencies]
asdcplib = { git = "https://github.com/PostPerfection/asdcplib-rs.git", branch = "master" }
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

- JPEG 2000 (mono + stereoscopic 3D)
- PCM audio (24-bit, 48kHz / 96kHz)
- Timed Text (SMPTE ST 429-5)
- Dolby Atmos (IAB)

## Building

The `asdcplib-sys` crate expects the asdcplib source tree at `asdcplib-sys/asdcplib/`. Clone it as a submodule:

```bash
git submodule add https://github.com/cinecert/asdcplib.git asdcplib-sys/asdcplib
cargo build
```

## License

BSD-3-Clause (matches asdcplib upstream)

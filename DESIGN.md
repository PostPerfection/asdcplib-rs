# Design

Safe Rust bindings for asdcplib (SMPTE AS-DCP / AS-02 MXF).

## Crates

- `asdcplib-sys`: raw FFI. C shim (`shim/asdcp_shim.cpp`) wraps ASDCP::/AS_02:: classes in extern "C" functions. `build.rs` builds vendored asdcplib via CMake and links it statically.
- `asdcplib`: safe wrappers over the sys crate.

## Coverage

- AS-DCP writers/readers: JP2K (mono + stereo), PCM, timed text, Atmos.
- AS-02 (IMF) writers/readers: JP2K, PCM (plain or with ST 377-4 MCA labels and the IMF MCA ChannelAssignment UL), timed text.
- Crypto contexts (AES encryption, HMAC) plumbed through writer/reader options, with an encrypted JP2K roundtrip test.
- `essence_type` probe and library version.

## Picture descriptor

`jp2k::PictureDescriptor` carries a `codestream: CodestreamHeader` holding the SIZ component count, image and tile grid, COD, QCD and CAP values that go into the MXF `JPEG2000PictureSubDescriptor`. `CodestreamHeader` is `#[non_exhaustive]` and has no `Default`, so `CodestreamHeader::parse(&frame_bytes)` is the only way a caller outside the crate gets one, and the sub-descriptor cannot be written zeroed.

The AS-02 writer derives the RGBA essence descriptor from that header: `PictureEssenceCoding` from Rsize (the cinema profiles map to the 2K/4K labels, the IMF profiles to the label for their main and sub level where MDD.cpp names one and to the generic per-family Lossy or Reversible label otherwise, everything else to Broadcast Profile 1), `PixelLayout` and `ComponentMaxRef` from the first component's bit depth, and the sub-descriptor's `J2CLayout` from Csiz and each component's precision. Depths other than 8, 10, 12 and 16 are refused. `VideoLineMap` is the only descriptor item the codestream cannot supply, and the writer always sets 1,0: SMPTE ST 377-1 puts 0 in the second entry for a progressive image, and a full-frame image starts at line 1.

Photon reads PixelBitDepth out of the J2CLayout components, so a file without J2CLayout fails its ST 2067-21 image characteristics check even when PixelLayout is correct. MXF has no item named PixelBitDepth.

`as02::jp2k::MxfReader::rgba_essence_descriptor` and `jpeg2000_sub_descriptor` read back every item of `MXF::RGBAEssenceDescriptor` and `MXF::JPEG2000PictureSubDescriptor`, both InstanceIDs included, which is what an IMF CPL EssenceDescriptorList has to repeat verbatim. `rgba_descriptor` is the older four-item view of the same read.

The AS-DCP writer sets its own `PictureEssenceCoding` and forces Rsize to 3 or 4 by stored width, inside asdcplib, so a DCP always signals a cinema profile whatever the codestream says.

## Testing

61 tests + 1 doctest, byte-exact MXF roundtrips through the real C++ library for all six reader/writer pairs, plus an encrypted (AES + HMAC) JP2K roundtrip. `asdcplib/tests/fixtures` holds real JPEG 2000 codestreams, since a picture descriptor can only be built by parsing one.

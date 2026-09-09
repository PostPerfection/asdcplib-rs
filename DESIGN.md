# Design

Safe Rust bindings for asdcplib (SMPTE AS-DCP / AS-02 MXF).

## Crates

- `asdcplib-sys`: raw FFI. C shim (`shim/asdcp_shim.cpp`) wraps ASDCP::/AS_02:: classes in extern "C" functions. `build.rs` builds vendored asdcplib via CMake and links it statically.
- `asdcplib`: safe wrappers over the sys crate.

## Coverage

- AS-DCP writers/readers: JP2K (mono + stereo), PCM, timed text, Atmos.
- AS-02 (IMF) writers/readers: JP2K, PCM (plain or with ST 377-4 MCA labels and the IMF MCA ChannelAssignment UL), timed text.

## AS-02 audio labels

`as02::pcm::MxfWriter::open_write_mca` takes an as-02-wrap style config string plus a `SoundfieldGroupProperties`. asdcplib's `AS02_MCAConfigParser` builds the label subdescriptors from the config, and the shim then sets `RFC5646SpokenLanguage`, `MCATitle`, `MCATitleVersion`, `MCAAudioContentKind` and `MCAAudioElementKind` on the one SoundfieldGroupLabelSubDescriptor the config produced, which is what ST 2067-2 section 5.3.6.5 requires and what Photon's `IMFConstraints.checkIMFCompliance` tests for non-emptiness. It tests presence only: the four are free text and Photon compares them against no registry, so the crate ships no value table. A config naming no soundfield group, or more than one, is refused, as is an empty property.
- Crypto contexts (AES encryption, HMAC) plumbed through writer/reader options, with an encrypted JP2K roundtrip test.
- `essence_type` probe and library version.

## Picture descriptor

`jp2k::PictureDescriptor` carries a `codestream: CodestreamHeader` holding the SIZ component count, image and tile grid, COD, QCD and CAP values that go into the MXF `JPEG2000PictureSubDescriptor`. `CodestreamHeader` is `#[non_exhaustive]` and has no `Default`, so `CodestreamHeader::parse(&frame_bytes)` is the only way a caller outside the crate gets one, and the sub-descriptor cannot be written zeroed.

The AS-02 writer derives the RGBA essence descriptor from that header: `PictureEssenceCoding` from Rsize (the cinema profiles map to the 2K/4K labels, the IMF profiles to the label for their main and sub level where MDD.cpp names one and to the generic per-family Lossy or Reversible label otherwise, everything else to Broadcast Profile 1), `PixelLayout` and `ComponentMaxRef` from the first component's bit depth, and the sub-descriptor's `J2CLayout` from Csiz and each component's precision. Depths other than 8, 10, 12 and 16 are refused. `VideoLineMap` is the only descriptor item the codestream cannot supply, and the writer always sets 1,0: SMPTE ST 377-1 puts 0 in the second entry for a progressive image, and a full-frame image starts at line 1.

Photon reads PixelBitDepth out of the J2CLayout components, so a file without J2CLayout fails its ST 2067-21 image characteristics check even when PixelLayout is correct. MXF has no item named PixelBitDepth.

`as02::jp2k::MxfReader::rgba_essence_descriptor` and `jpeg2000_sub_descriptor` read back every item of `MXF::RGBAEssenceDescriptor` and `MXF::JPEG2000PictureSubDescriptor`, both InstanceIDs included, which is what an IMF CPL EssenceDescriptorList has to repeat verbatim. `rgba_descriptor` is the older four-item view of the same read.

The AS-DCP writer sets its own `PictureEssenceCoding` and forces Rsize to 3 or 4 by stored width, inside asdcplib, so a DCP always signals a cinema profile whatever the codestream says.

## Sound descriptor

`as02::pcm::MxfReader::wave_audio_descriptor` reads back every item of `MXF::WaveAudioDescriptor` and of the `FileDescriptor` and `GenericSoundEssenceDescriptor` it derives from, the InstanceID and the SubDescriptors list included, which is what an IMF CPL EssenceDescriptorList has to repeat verbatim. `pcm::McaLabelSubDescriptor` carries each label's own InstanceID beside its MCALinkID, so the entry can name the same subdescriptors the SubDescriptors list points at. `audio_descriptor` is the older ten-item view of the same read.

Four items come from the writer rather than from the caller's `AudioDescriptor`: `SampleRate` is set to `AudioSamplingRate` and `ContainerDuration` counts samples, both because AS-02 PCM is clip-wrapped, `EssenceContainer` is the ST 382 clip-wrapped WAVE UL, and `LinkedTrackID` is 1, the only track in the file package. `SoundEssenceCoding` is mandatory in ST 377-1 and asdcplib never sets it, so it reads back nil.

## Timed text descriptor

`timed_text::TimedTextDescriptor` carries `namespace_uri` and `ucs_encoding` beside the edit rate, container duration and asset id. The writer stamps them into the MXF descriptor's `NamespaceURI` and `UCSEncoding`, and the reader reads them back, so an IMF caller can set the IMSC profile designator (e.g. `http://www.w3.org/ns/ttml/profile/imsc1/text`) that Photon requires and repeat it in the CPL EssenceDescriptorList. Both default to the empty string asdcplib wrote before, which Photon rejects.

## Testing

65 tests + 1 doctest, byte-exact MXF roundtrips through the real C++ library for all six reader/writer pairs, plus an encrypted (AES + HMAC) JP2K roundtrip. `asdcplib/tests/fixtures` holds real JPEG 2000 codestreams, since a picture descriptor can only be built by parsing one.

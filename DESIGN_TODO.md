# Planned

(none)

## Done

HDR/WCG picture metadata on the AS-02 (IMF) and AS-DCP JP2K writers (2026-07-23),
for ST 2067-21 HDR essence (imfwizard). One options struct `HdrMetadata` carries
transfer characteristic, color primaries and the ST 2086 mastering display block
(display primaries, white point, max/min luminance). The shim gained
`asdcp_as02_jp2k_writer_open_write_hdr` (set the fields on the RGBA descriptor
before OpenWrite: AS-02 SetSourceStream writes the header during OpenWrite and
WriteAS02Footer rewrites it at Finalize, both from the retained descriptor),
`asdcp_jp2k_writer_open_write_hdr` (AS-DCP, set after OpenWrite like the transfer
setter), and `asdcp_{as02_,}jp2k_reader_read_hdr` (read every field back off the
GenericPictureEssenceDescriptor). Safe wrappers: `open_write_hdr` and
`hdr_metadata()` on both `jp2k` and `as02::jp2k` writers/readers. New constants
`jp2k::{TRANSFER_CHARACTERISTIC_BT2020, COLOR_PRIMARIES_BT709, COLOR_PRIMARIES_BT2020,
COLOR_PRIMARIES_P3D65}` (ULs read from MDD.cpp). Roundtrip tested for AS-02 and
AS-DCP with ST 2084 + BT.2020/P3D65 primaries + full mastering display, plus
absent-metadata cases.

Not supported by the vendored asdcplib: MaxCLL (MaximumContentLightLevel) and
MaxFALL (MaximumFrameAverageLightLevel). No such property exists on
`GenericPictureEssenceDescriptor` (Metadata.h) or anywhere in the vendored tree,
and no MDD entry defines their ULs, so they are not settable without patching the
C++. Everything else ST 2067-21 needs for HDR essence descriptors is present.

TransferCharacteristic UL on the AS-DCP JP2K writer (2026-07-23), for HDR DCI
Addendum DCPs (dcpwizard --hdr-dci). The shim gained
`asdcp_jp2k_writer_open_write_transfer` (OpenWrite, then set the 16-byte UL on
the RGBAEssenceDescriptor the writer created; the header is rewritten at Finalize
so the post-OpenWrite change persists) and
`asdcp_jp2k_reader_read_transfer_characteristic` (read the UL back off the
descriptor, reporting present/absent). Safe wrappers:
`jp2k::MxfWriter::open_write_transfer` and `jp2k::MxfReader::transfer_characteristic`
returning `Option<[u8; 16]>`, plus the `jp2k::TRANSFER_CHARACTERISTIC_ST2084`
constant (SMPTE ST 2084 PQ UL). Roundtrip tested with the ST 2084 UL, plus an
absent-property case.

SMPTE 377-4 MCA labels on the AS-DCP PCM writer (2026-07-21). The shim gained
`asdcp_pcm_writer_open_write_mca` (parse an asdcp-wrap style config string with
`ASDCP::MXF::ASDCP_MCAConfigParser`, then after OpenWrite add each subdescriptor
to the OP1a header and link it from the WaveAudioDescriptor with the MCA
ChannelAssignment UL, replicating asdcp-wrap.cpp including the `*i = 0` ownership
transfer) and `asdcp_pcm_reader_read_mca_labels` (count AudioChannelLabel and
SoundfieldGroup subdescriptors and report whether the MCA ChannelAssignment UL is
present). Safe wrappers: `pcm::MxfWriter::open_write_mca` and
`pcm::MxfReader::mca_labels` returning `McaLabelSummary`. Roundtrip tested for
5.1, plus a channel-count-mismatch failure. Stereoscopic (JP2K MXFS) and Atmos
(ATMOS DCData) were already bound; no change needed there.

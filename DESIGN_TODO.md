# Planned

(none)

# Done

## 2026-08-12

Per-subdescriptor MCA label reading on the AS-DCP PCM reader, so a caller can
tell a visually impaired narration channel from a hearing impaired channel from
a sign language video channel instead of only counting labels.

Shim entry points `asdcp_pcm_reader_mca_label_count` and
`asdcp_pcm_reader_mca_label_info`, following the count-plus-index convention the
timed-text ancillary resource reader already uses. The walk goes through the
WaveAudioDescriptor's SubDescriptors array rather than the header's packet list,
because only that array promises the order the labels were written in. Each entry
fills a caller-owned `asdcp_mca_label_t`: kind (audio channel, soundfield group,
group of soundfield groups), MCATagSymbol, MCALabelDictionaryID, MCALinkID, and
the optional MCATagName, MCAChannelID, RFC5646SpokenLanguage and
SoundfieldGroupLinkID. Its string fields are fixed at 129 bytes because
`UTF16String::Archive` refuses to write an MCA string longer than 128.

Safe wrapper `pcm::MxfReader::mca_label_subdescriptors` returning
`Vec<McaLabelSubDescriptor>` with `Option` fields and a `McaLabelKind` enum. The
existing `asdcp_pcm_reader_read_mca_labels`, `mca_labels` and `McaLabelSummary`
are unchanged. Roundtrip tested for 5.1 (exact tag symbols in wrap order,
one-based channel ids, every channel linked to the soundfield group), for the
accessibility channels HI, VIN and SLVS, and for an MXF with no MCA labels.

Left out because nothing can write them: SoundfieldGroupLabelSubDescriptor's
GroupOfSoundfieldGroupsLinkID and the remaining MCALabelSubDescriptor optional
properties (MCATitle, MCATitleVersion, MCATitleSubVersion, MCAEpisode,
MCAPartitionKind, MCAPartitionNumber, MCAAudioContentKind, MCAAudioElementKind).
All exist on the vendored classes, but `ASDCP_MCAConfigParser` never sets any of
them, so the writer side of this repo cannot produce a file that exercises them.

Also a vendored limit rather than a binding gap: the AS-DCP label vocabulary is
the fixed map in MXF.cpp, so HI, VIN and SLVS are the only accessibility symbols
`ASDCP_MCAConfigParser` accepts. The IMF-only HA and VA symbols live in
`AS02_MCAConfigParser`, which the AS-DCP PCM writer does not use.

## 2026-07-23

HDR/WCG picture metadata on the AS-DCP and AS-02 (IMF) JP2K writers/readers
(6d7b8ca), for ST 2067-21 HDR essence (imfwizard) and HDR DCI Addendum DCPs
(dcpwizard --hdr-dci).

One `HdrMetadata` struct carries transfer characteristic, color primaries and the
ST 2086 mastering display block (display primaries, white point, max/min luminance).
Shim entry points: `asdcp_as02_jp2k_writer_open_write_hdr` (AS-02 SetSourceStream
writes the header at OpenWrite and WriteAS02Footer rewrites it at Finalize, both from
the retained descriptor), `asdcp_jp2k_writer_open_write_hdr` (AS-DCP, set after
OpenWrite), and `asdcp_{as02_,}jp2k_reader_read_hdr` (read every field off the
GenericPictureEssenceDescriptor). Safe wrappers `open_write_hdr` and `hdr_metadata()`
on both `jp2k` and `as02::jp2k`. AS-DCP also gained a transfer-only path
(`asdcp_jp2k_writer_open_write_transfer` / `open_write_transfer` +
`transfer_characteristic()`) for DCPs that need only the TransferCharacteristic UL.
New constants `jp2k::{TRANSFER_CHARACTERISTIC_ST2084, TRANSFER_CHARACTERISTIC_BT2020,
COLOR_PRIMARIES_BT709, COLOR_PRIMARIES_BT2020, COLOR_PRIMARIES_P3D65}` (ULs read from
MDD.cpp). Roundtrip tested for AS-02 and AS-DCP with ST 2084 + BT.2020/P3D65 + full
mastering display, plus absent-metadata cases.

Not supported by the vendored asdcplib: MaxCLL (MaximumContentLightLevel) and MaxFALL
(MaximumFrameAverageLightLevel). No such property exists on
`GenericPictureEssenceDescriptor` (Metadata.h) or anywhere in the vendored tree, and
no MDD entry defines their ULs, so they are not settable without patching the C++.
Everything else ST 2067-21 needs for HDR essence descriptors is present.

## 2026-07-22

Timed-text ancillary resource reader (66de9d0) on the AS-DCP TimedText reader.

## 2026-07-21

SMPTE 377-4 MCA labels on the AS-DCP PCM writer (5fe4d61). The shim gained
`asdcp_pcm_writer_open_write_mca` (parse an asdcp-wrap style config string with
`ASDCP::MXF::ASDCP_MCAConfigParser`, then after OpenWrite add each subdescriptor to
the OP1a header and link it from the WaveAudioDescriptor with the MCA
ChannelAssignment UL, replicating asdcp-wrap.cpp including the `*i = 0` ownership
transfer) and `asdcp_pcm_reader_read_mca_labels` (count AudioChannelLabel and
SoundfieldGroup subdescriptors, report whether the MCA ChannelAssignment UL is
present). Safe wrappers `pcm::MxfWriter::open_write_mca` and `pcm::MxfReader::mca_labels`
returning `McaLabelSummary`. Roundtrip tested for 5.1, plus a channel-count-mismatch
failure. Stereoscopic (JP2K MXFS) and Atmos (ATMOS DCData) were already bound.

# Planned

(none)

## Done

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

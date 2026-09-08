/* asdcplib C shim implementation */
#include "asdcp_shim.h"
#include <AS_DCP.h>
#include <AS_02.h>
#include <Metadata.h>
#include <MXF.h>
#include <MDD.h>
#include <KM_fileio.h>
#include <KM_memio.h>
#include <cstring>
#include <string>
#include <list>

/* Internal descriptor converters live in AS_DCP_internal.h, which is not part
   of the public include surface. Forward-declare them exactly as the upstream
   as-02-wrap / as-02-unwrap tools do so the linker resolves them. */
namespace ASDCP {
  Kumu::Result_t JP2K_PDesc_to_MD(const ASDCP::JP2K::PictureDescriptor& PDesc,
                                  const ASDCP::Dictionary& dict,
                                  ASDCP::MXF::GenericPictureEssenceDescriptor& EssenceDescriptor,
                                  ASDCP::MXF::JPEG2000PictureSubDescriptor& EssenceSubDescriptor);
  Kumu::Result_t MD_to_JP2K_PDesc(const ASDCP::MXF::GenericPictureEssenceDescriptor& EssenceDescriptor,
                                  const ASDCP::MXF::JPEG2000PictureSubDescriptor& EssenceSubDescriptor,
                                  const ASDCP::Rational& EditRate, const ASDCP::Rational& SampleRate,
                                  ASDCP::JP2K::PictureDescriptor& PDesc);
  Kumu::Result_t PCM_ADesc_to_MD(ASDCP::PCM::AudioDescriptor& ADesc, ASDCP::MXF::WaveAudioDescriptor* ADescObj);
  Kumu::Result_t MD_to_PCM_ADesc(ASDCP::MXF::WaveAudioDescriptor* ADescObj, ASDCP::PCM::AudioDescriptor& ADesc);
}

/* Helper: convert C writer info to C++ WriterInfo */
static void c_to_cpp_writer_info(const asdcp_writer_info_t* c, ASDCP::WriterInfo& cpp) {
    memcpy(cpp.ProductUUID, c->product_uuid, 16);
    memcpy(cpp.AssetUUID, c->asset_uuid, 16);
    memcpy(cpp.ContextID, c->context_id, 16);
    memcpy(cpp.CryptographicKeyID, c->cryptographic_key_id, 16);
    cpp.EncryptedEssence = (c->encrypted_essence != 0);
    cpp.UsesHMAC = (c->uses_hmac != 0);
    cpp.LabelSetType = static_cast<ASDCP::LabelSet_t>(c->label_set_type);
}

/* Helper: convert C++ WriterInfo to C */
static void cpp_to_c_writer_info(const ASDCP::WriterInfo& cpp, asdcp_writer_info_t* c) {
    memcpy(c->product_uuid, cpp.ProductUUID, 16);
    memcpy(c->asset_uuid, cpp.AssetUUID, 16);
    memcpy(c->context_id, cpp.ContextID, 16);
    memcpy(c->cryptographic_key_id, cpp.CryptographicKeyID, 16);
    c->encrypted_essence = cpp.EncryptedEssence ? 1 : 0;
    c->uses_hmac = cpp.UsesHMAC ? 1 : 0;
    c->label_set_type = static_cast<int32_t>(cpp.LabelSetType);
}

static void c_to_cpp_codestream_header(const asdcp_codestream_header_t* c,
    ASDCP::JP2K::PictureDescriptor& cpp) {
    cpp.Rsize = c->rsize;
    cpp.Xsize = c->xsize;
    cpp.Ysize = c->ysize;
    cpp.XOsize = c->x_osize;
    cpp.YOsize = c->y_osize;
    cpp.XTsize = c->xt_size;
    cpp.YTsize = c->yt_size;
    cpp.XTOsize = c->xt_osize;
    cpp.YTOsize = c->yt_osize;
    cpp.Csize = c->csize;

    for (uint32_t i = 0; i < ASDCP_JP2K_MAX_COMPONENTS; ++i) {
        cpp.ImageComponents[i].Ssize = c->image_components[i].ssize;
        cpp.ImageComponents[i].XRsize = c->image_components[i].x_rsize;
        cpp.ImageComponents[i].YRsize = c->image_components[i].y_rsize;
    }

    const asdcp_coding_style_default_t* cod = &c->coding_style_default;
    cpp.CodingStyleDefault.Scod = cod->scod;
    cpp.CodingStyleDefault.SGcod.ProgressionOrder = cod->progression_order;
    cpp.CodingStyleDefault.SGcod.NumberOfLayers[0] = cod->number_of_layers[0];
    cpp.CodingStyleDefault.SGcod.NumberOfLayers[1] = cod->number_of_layers[1];
    cpp.CodingStyleDefault.SGcod.MultiCompTransform = cod->multi_component_transform;
    cpp.CodingStyleDefault.SPcod.DecompositionLevels = cod->decomposition_levels;
    cpp.CodingStyleDefault.SPcod.CodeblockWidth = cod->codeblock_width;
    cpp.CodingStyleDefault.SPcod.CodeblockHeight = cod->codeblock_height;
    cpp.CodingStyleDefault.SPcod.CodeblockStyle = cod->codeblock_style;
    cpp.CodingStyleDefault.SPcod.Transformation = cod->transformation;
    memcpy(cpp.CodingStyleDefault.SPcod.PrecinctSize, cod->precinct_sizes,
           ASDCP_JP2K_MAX_PRECINCT_SIZES);

    cpp.QuantizationDefault.Sqcd = c->quantization_default.sqcd;
    memcpy(cpp.QuantizationDefault.SPqcd, c->quantization_default.spqcd,
           ASDCP_JP2K_MAX_QUANTIZATION_STEPS);
    cpp.QuantizationDefault.SPqcdLength = c->quantization_default.spqcd_length;

    cpp.ExtendedCapabilities.Pcap = c->extended_capabilities.pcap;
    cpp.ExtendedCapabilities.N = c->extended_capabilities.capability_count;
    for (uint32_t i = 0; i < ASDCP_JP2K_MAX_CAPABILITIES; ++i) {
        cpp.ExtendedCapabilities.Ccap[i] = c->extended_capabilities.ccap[i];
    }
}

static void cpp_to_c_codestream_header(const ASDCP::JP2K::PictureDescriptor& cpp,
    asdcp_codestream_header_t* c) {
    memset(c, 0, sizeof(*c));
    c->rsize = cpp.Rsize;
    c->xsize = cpp.Xsize;
    c->ysize = cpp.Ysize;
    c->x_osize = cpp.XOsize;
    c->y_osize = cpp.YOsize;
    c->xt_size = cpp.XTsize;
    c->yt_size = cpp.YTsize;
    c->xt_osize = cpp.XTOsize;
    c->yt_osize = cpp.YTOsize;
    c->csize = cpp.Csize;

    for (uint32_t i = 0; i < ASDCP_JP2K_MAX_COMPONENTS; ++i) {
        c->image_components[i].ssize = cpp.ImageComponents[i].Ssize;
        c->image_components[i].x_rsize = cpp.ImageComponents[i].XRsize;
        c->image_components[i].y_rsize = cpp.ImageComponents[i].YRsize;
    }

    asdcp_coding_style_default_t* cod = &c->coding_style_default;
    cod->scod = cpp.CodingStyleDefault.Scod;
    cod->progression_order = cpp.CodingStyleDefault.SGcod.ProgressionOrder;
    cod->number_of_layers[0] = cpp.CodingStyleDefault.SGcod.NumberOfLayers[0];
    cod->number_of_layers[1] = cpp.CodingStyleDefault.SGcod.NumberOfLayers[1];
    cod->multi_component_transform = cpp.CodingStyleDefault.SGcod.MultiCompTransform;
    cod->decomposition_levels = cpp.CodingStyleDefault.SPcod.DecompositionLevels;
    cod->codeblock_width = cpp.CodingStyleDefault.SPcod.CodeblockWidth;
    cod->codeblock_height = cpp.CodingStyleDefault.SPcod.CodeblockHeight;
    cod->codeblock_style = cpp.CodingStyleDefault.SPcod.CodeblockStyle;
    cod->transformation = cpp.CodingStyleDefault.SPcod.Transformation;
    memcpy(cod->precinct_sizes, cpp.CodingStyleDefault.SPcod.PrecinctSize,
           ASDCP_JP2K_MAX_PRECINCT_SIZES);

    c->quantization_default.sqcd = cpp.QuantizationDefault.Sqcd;
    memcpy(c->quantization_default.spqcd, cpp.QuantizationDefault.SPqcd,
           ASDCP_JP2K_MAX_QUANTIZATION_STEPS);
    c->quantization_default.spqcd_length = cpp.QuantizationDefault.SPqcdLength;

    c->extended_capabilities.pcap = cpp.ExtendedCapabilities.Pcap;
    c->extended_capabilities.capability_count = cpp.ExtendedCapabilities.N;
    for (uint32_t i = 0; i < ASDCP_JP2K_MAX_CAPABILITIES; ++i) {
        c->extended_capabilities.ccap[i] = cpp.ExtendedCapabilities.Ccap[i];
    }
}

asdcp_result_t asdcp_jp2k_parse_codestream_header(const uint8_t* codestream, uint32_t size,
    asdcp_codestream_header_t* out) {
    ASDCP::JP2K::FrameBuffer fb;
    fb.SetData(const_cast<uint8_t*>(codestream), size);
    fb.Size(size);

    ASDCP::JP2K::PictureDescriptor pd = ASDCP::JP2K::PictureDescriptor();
    ASDCP::Result_t result = ASDCP::JP2K::ParseMetadataIntoDesc(fb, pd);
    if (ASDCP_SUCCESS(result)) {
        cpp_to_c_codestream_header(pd, out);
    }
    return result.Value();
}

/* Helper: convert C picture desc to C++ */
static void c_to_cpp_picture_desc(const asdcp_picture_descriptor_t* c, ASDCP::JP2K::PictureDescriptor& cpp) {
    cpp = ASDCP::JP2K::PictureDescriptor();
    cpp.EditRate = ASDCP::Rational(c->edit_rate.numerator, c->edit_rate.denominator);
    cpp.SampleRate = ASDCP::Rational(c->sample_rate.numerator, c->sample_rate.denominator);
    cpp.StoredWidth = c->stored_width;
    cpp.StoredHeight = c->stored_height;
    cpp.AspectRatio = ASDCP::Rational(c->aspect_ratio.numerator, c->aspect_ratio.denominator);
    cpp.ContainerDuration = c->container_duration;
    c_to_cpp_codestream_header(&c->codestream, cpp);
}

static void cpp_to_c_picture_desc(const ASDCP::JP2K::PictureDescriptor& cpp, asdcp_picture_descriptor_t* c) {
    cpp_to_c_codestream_header(cpp, &c->codestream);
    c->edit_rate.numerator = cpp.EditRate.Numerator;
    c->edit_rate.denominator = cpp.EditRate.Denominator;
    c->sample_rate.numerator = cpp.SampleRate.Numerator;
    c->sample_rate.denominator = cpp.SampleRate.Denominator;
    c->stored_width = cpp.StoredWidth;
    c->stored_height = cpp.StoredHeight;
    c->aspect_ratio.numerator = cpp.AspectRatio.Numerator;
    c->aspect_ratio.denominator = cpp.AspectRatio.Denominator;
    c->container_duration = cpp.ContainerDuration;
}

static void c_to_cpp_audio_desc(const asdcp_audio_descriptor_t* c, ASDCP::PCM::AudioDescriptor& cpp) {
    cpp.EditRate = ASDCP::Rational(c->edit_rate.numerator, c->edit_rate.denominator);
    cpp.AudioSamplingRate = ASDCP::Rational(c->audio_sampling_rate.numerator, c->audio_sampling_rate.denominator);
    cpp.Locked = c->locked;
    cpp.ChannelCount = c->channel_count;
    cpp.QuantizationBits = c->quantization_bits;
    cpp.BlockAlign = c->block_align;
    cpp.AvgBps = c->avg_bps;
    cpp.LinkedTrackID = c->linked_track_id;
    cpp.ContainerDuration = c->container_duration;
    cpp.ChannelFormat = static_cast<ASDCP::PCM::ChannelFormat_t>(c->channel_format);
}

static void cpp_to_c_audio_desc(const ASDCP::PCM::AudioDescriptor& cpp, asdcp_audio_descriptor_t* c) {
    c->edit_rate.numerator = cpp.EditRate.Numerator;
    c->edit_rate.denominator = cpp.EditRate.Denominator;
    c->audio_sampling_rate.numerator = cpp.AudioSamplingRate.Numerator;
    c->audio_sampling_rate.denominator = cpp.AudioSamplingRate.Denominator;
    c->locked = cpp.Locked;
    c->channel_count = cpp.ChannelCount;
    c->quantization_bits = cpp.QuantizationBits;
    c->block_align = cpp.BlockAlign;
    c->avg_bps = cpp.AvgBps;
    c->linked_track_id = cpp.LinkedTrackID;
    c->container_duration = cpp.ContainerDuration;
    c->channel_format = static_cast<int32_t>(cpp.ChannelFormat);
}

static void c_to_cpp_timed_text_desc(const asdcp_timed_text_descriptor_t* c, ASDCP::TimedText::TimedTextDescriptor& cpp) {
    cpp.EditRate = ASDCP::Rational(c->edit_rate.numerator, c->edit_rate.denominator);
    cpp.ContainerDuration = c->container_duration;
    memcpy(cpp.AssetID, c->asset_id, 16);
}

static void cpp_to_c_timed_text_desc(const ASDCP::TimedText::TimedTextDescriptor& cpp, asdcp_timed_text_descriptor_t* c) {
    c->edit_rate.numerator = cpp.EditRate.Numerator;
    c->edit_rate.denominator = cpp.EditRate.Denominator;
    c->container_duration = cpp.ContainerDuration;
    memcpy(c->asset_id, cpp.AssetID, 16);
}

static void c_to_cpp_atmos_desc(const asdcp_atmos_descriptor_t* c, ASDCP::ATMOS::AtmosDescriptor& cpp) {
    cpp.EditRate = ASDCP::Rational(c->edit_rate.numerator, c->edit_rate.denominator);
    cpp.ContainerDuration = c->container_duration;
    memcpy(cpp.AssetID, c->asset_id, 16);
    memcpy(cpp.DataEssenceCoding, c->data_essence_coding, 16);
    cpp.FirstFrame = c->first_frame;
    cpp.MaxChannelCount = c->max_channel_count;
    cpp.MaxObjectCount = c->max_object_count;
    memcpy(cpp.AtmosID, c->atmos_id, 16);
    cpp.AtmosVersion = c->atmos_version;
}

static void cpp_to_c_atmos_desc(const ASDCP::ATMOS::AtmosDescriptor& cpp, asdcp_atmos_descriptor_t* c) {
    c->edit_rate.numerator = cpp.EditRate.Numerator;
    c->edit_rate.denominator = cpp.EditRate.Denominator;
    c->container_duration = cpp.ContainerDuration;
    memcpy(c->asset_id, cpp.AssetID, 16);
    memcpy(c->data_essence_coding, cpp.DataEssenceCoding, 16);
    c->first_frame = cpp.FirstFrame;
    c->max_channel_count = cpp.MaxChannelCount;
    c->max_object_count = cpp.MaxObjectCount;
    memcpy(c->atmos_id, cpp.AtmosID, 16);
    c->atmos_version = cpp.AtmosVersion;
}

/* Apply HDR/WCG metadata to a picture essence descriptor. Only fields with a set
   presence flag are written, so unset optional properties stay absent. */
static void apply_hdr_metadata(ASDCP::MXF::GenericPictureEssenceDescriptor* ed,
    const asdcp_hdr_metadata_t* hdr) {
    if (hdr->has_transfer_characteristic) {
        ed->TransferCharacteristic = ASDCP::UL(hdr->transfer_characteristic);
    }
    if (hdr->has_color_primaries) {
        ed->ColorPrimaries = ASDCP::UL(hdr->color_primaries);
    }
    if (hdr->has_mastering_display_primaries) {
        const uint16_t* p = hdr->mastering_display_primaries;
        ed->MasteringDisplayPrimaries = ASDCP::MXF::ThreeColorPrimaries(
            ASDCP::MXF::ColorPrimary(p[0], p[1]),
            ASDCP::MXF::ColorPrimary(p[2], p[3]),
            ASDCP::MXF::ColorPrimary(p[4], p[5]));
    }
    if (hdr->has_mastering_display_white_point) {
        ed->MasteringDisplayWhitePointChromaticity = ASDCP::MXF::ColorPrimary(
            hdr->mastering_display_white_point[0], hdr->mastering_display_white_point[1]);
    }
    if (hdr->has_mastering_display_max_luminance) {
        ed->MasteringDisplayMaximumLuminance = hdr->mastering_display_max_luminance;
    }
    if (hdr->has_mastering_display_min_luminance) {
        ed->MasteringDisplayMinimumLuminance = hdr->mastering_display_min_luminance;
    }
}

/* Read HDR/WCG metadata off a picture essence descriptor into the C struct. */
static void read_hdr_metadata(const ASDCP::MXF::GenericPictureEssenceDescriptor* ed,
    asdcp_hdr_metadata_t* hdr) {
    memset(hdr, 0, sizeof(*hdr));
    if (!ed->TransferCharacteristic.empty()) {
        memcpy(hdr->transfer_characteristic, ed->TransferCharacteristic.const_get().Value(), 16);
        hdr->has_transfer_characteristic = 1;
    }
    if (!ed->ColorPrimaries.empty()) {
        memcpy(hdr->color_primaries, ed->ColorPrimaries.const_get().Value(), 16);
        hdr->has_color_primaries = 1;
    }
    if (!ed->MasteringDisplayPrimaries.empty()) {
        const ASDCP::MXF::ThreeColorPrimaries& tcp = ed->MasteringDisplayPrimaries.const_get();
        hdr->mastering_display_primaries[0] = tcp.First.X;
        hdr->mastering_display_primaries[1] = tcp.First.Y;
        hdr->mastering_display_primaries[2] = tcp.Second.X;
        hdr->mastering_display_primaries[3] = tcp.Second.Y;
        hdr->mastering_display_primaries[4] = tcp.Third.X;
        hdr->mastering_display_primaries[5] = tcp.Third.Y;
        hdr->has_mastering_display_primaries = 1;
    }
    if (!ed->MasteringDisplayWhitePointChromaticity.empty()) {
        const ASDCP::MXF::ColorPrimary& wp = ed->MasteringDisplayWhitePointChromaticity.const_get();
        hdr->mastering_display_white_point[0] = wp.X;
        hdr->mastering_display_white_point[1] = wp.Y;
        hdr->has_mastering_display_white_point = 1;
    }
    if (!ed->MasteringDisplayMaximumLuminance.empty()) {
        hdr->mastering_display_max_luminance = ed->MasteringDisplayMaximumLuminance.const_get();
        hdr->has_mastering_display_max_luminance = 1;
    }
    if (!ed->MasteringDisplayMinimumLuminance.empty()) {
        hdr->mastering_display_min_luminance = ed->MasteringDisplayMinimumLuminance.const_get();
        hdr->has_mastering_display_min_luminance = 1;
    }
}

/* ---- Version ---- */
const char* asdcp_version(void) {
    return ASDCP::Version();
}

/* ---- Essence type detection ---- */
asdcp_result_t asdcp_essence_type(const char* filename, int32_t* out_type) {
    ASDCP::EssenceType_t type = ASDCP::ESS_UNKNOWN;
    Kumu::FileReaderFactory defaultFactory;
    ASDCP::Result_t r = ASDCP::EssenceType(std::string(filename), type, defaultFactory);
    *out_type = static_cast<int32_t>(type);
    return r.Value();
}

asdcp_result_t asdcp_raw_essence_type(const char* filename, int32_t* out_type) {
    ASDCP::EssenceType_t type = ASDCP::ESS_UNKNOWN;
    ASDCP::Result_t r = ASDCP::RawEssenceType(std::string(filename), type);
    *out_type = static_cast<int32_t>(type);
    return r.Value();
}

/* ---- AES Encryption Context ---- */
asdcp_aes_enc_context_t asdcp_aes_enc_context_new(void) {
    return new ASDCP::AESEncContext();
}

void asdcp_aes_enc_context_free(asdcp_aes_enc_context_t ctx) {
    delete static_cast<ASDCP::AESEncContext*>(ctx);
}

asdcp_result_t asdcp_aes_enc_context_init_key(asdcp_aes_enc_context_t ctx, const uint8_t* key) {
    return static_cast<ASDCP::AESEncContext*>(ctx)->InitKey(key).Value();
}

asdcp_result_t asdcp_aes_enc_context_set_ivec(asdcp_aes_enc_context_t ctx, const uint8_t* ivec) {
    return static_cast<ASDCP::AESEncContext*>(ctx)->SetIVec(ivec).Value();
}

/* ---- AES Decryption Context ---- */
asdcp_aes_dec_context_t asdcp_aes_dec_context_new(void) {
    return new ASDCP::AESDecContext();
}

void asdcp_aes_dec_context_free(asdcp_aes_dec_context_t ctx) {
    delete static_cast<ASDCP::AESDecContext*>(ctx);
}

asdcp_result_t asdcp_aes_dec_context_init_key(asdcp_aes_dec_context_t ctx, const uint8_t* key) {
    return static_cast<ASDCP::AESDecContext*>(ctx)->InitKey(key).Value();
}

asdcp_result_t asdcp_aes_dec_context_set_ivec(asdcp_aes_dec_context_t ctx, const uint8_t* ivec) {
    return static_cast<ASDCP::AESDecContext*>(ctx)->SetIVec(ivec).Value();
}

/* ---- HMAC Context ---- */
asdcp_hmac_context_t asdcp_hmac_context_new(void) {
    return new ASDCP::HMACContext();
}

void asdcp_hmac_context_free(asdcp_hmac_context_t ctx) {
    delete static_cast<ASDCP::HMACContext*>(ctx);
}

asdcp_result_t asdcp_hmac_context_init_key(asdcp_hmac_context_t ctx, const uint8_t* key, int32_t label_set) {
    return static_cast<ASDCP::HMACContext*>(ctx)->InitKey(key, static_cast<ASDCP::LabelSet_t>(label_set)).Value();
}

/* ---- JP2K Writer ---- */
asdcp_jp2k_writer_t asdcp_jp2k_writer_new(void) {
    return new ASDCP::JP2K::MXFWriter();
}

void asdcp_jp2k_writer_free(asdcp_jp2k_writer_t w) {
    delete static_cast<ASDCP::JP2K::MXFWriter*>(w);
}

asdcp_result_t asdcp_jp2k_writer_open_write(asdcp_jp2k_writer_t w, const char* filename,
    const asdcp_writer_info_t* info, const asdcp_picture_descriptor_t* desc, uint32_t header_size) {
    ASDCP::WriterInfo wi;
    c_to_cpp_writer_info(info, wi);
    ASDCP::JP2K::PictureDescriptor pd;
    c_to_cpp_picture_desc(desc, pd);
    return static_cast<ASDCP::JP2K::MXFWriter*>(w)->OpenWrite(std::string(filename), wi, pd, header_size).Value();
}

asdcp_result_t asdcp_jp2k_writer_write_frame(asdcp_jp2k_writer_t w,
    const uint8_t* frame_data, uint32_t frame_size,
    asdcp_aes_enc_context_t enc_ctx, asdcp_hmac_context_t hmac_ctx) {
    ASDCP::JP2K::FrameBuffer fb;
    fb.SetData(const_cast<uint8_t*>(frame_data), frame_size);
    fb.Size(frame_size);
    return static_cast<ASDCP::JP2K::MXFWriter*>(w)->WriteFrame(
        fb,
        static_cast<ASDCP::AESEncContext*>(enc_ctx),
        static_cast<ASDCP::HMACContext*>(hmac_ctx)
    ).Value();
}

asdcp_result_t asdcp_jp2k_writer_finalize(asdcp_jp2k_writer_t w) {
    return static_cast<ASDCP::JP2K::MXFWriter*>(w)->Finalize().Value();
}

/* Open a JP2K MXF, then set the TransferCharacteristic UL on the RGBAEssenceDescriptor
   the writer created. h__ASDCPWriter rewrites the header at Finalize, so the change
   made here after OpenWrite persists. */
asdcp_result_t asdcp_jp2k_writer_open_write_transfer(asdcp_jp2k_writer_t w, const char* filename,
    const asdcp_writer_info_t* info, const asdcp_picture_descriptor_t* desc,
    const uint8_t* transfer_characteristic_ul, uint32_t header_size) {
    const ASDCP::Dictionary* dict = &ASDCP::DefaultSMPTEDict();
    ASDCP::JP2K::MXFWriter* writer = static_cast<ASDCP::JP2K::MXFWriter*>(w);

    ASDCP::WriterInfo wi;
    c_to_cpp_writer_info(info, wi);
    ASDCP::JP2K::PictureDescriptor pd;
    c_to_cpp_picture_desc(desc, pd);

    ASDCP::Result_t result = writer->OpenWrite(std::string(filename), wi, pd, header_size);
    if (ASDCP_FAILURE(result)) {
        return result.Value();
    }

    if (transfer_characteristic_ul != 0) {
        ASDCP::MXF::RGBAEssenceDescriptor* ed = 0;
        writer->OP1aHeader().GetMDObjectByType(dict->ul(ASDCP::MDD_RGBAEssenceDescriptor),
            reinterpret_cast<ASDCP::MXF::InterchangeObject**>(&ed));
        if (ed == 0) {
            return ASDCP::RESULT_FORMAT.Value();
        }
        ed->TransferCharacteristic = ASDCP::UL(transfer_characteristic_ul);
    }
    return result.Value();
}

/* Open a JP2K MXF, then set HDR/WCG metadata on the RGBAEssenceDescriptor the
   writer created. h__ASDCPWriter rewrites the header at Finalize, so the change
   made here after OpenWrite persists. */
asdcp_result_t asdcp_jp2k_writer_open_write_hdr(asdcp_jp2k_writer_t w, const char* filename,
    const asdcp_writer_info_t* info, const asdcp_picture_descriptor_t* desc,
    const asdcp_hdr_metadata_t* hdr, uint32_t header_size) {
    const ASDCP::Dictionary* dict = &ASDCP::DefaultSMPTEDict();
    ASDCP::JP2K::MXFWriter* writer = static_cast<ASDCP::JP2K::MXFWriter*>(w);

    ASDCP::WriterInfo wi;
    c_to_cpp_writer_info(info, wi);
    ASDCP::JP2K::PictureDescriptor pd;
    c_to_cpp_picture_desc(desc, pd);

    ASDCP::Result_t result = writer->OpenWrite(std::string(filename), wi, pd, header_size);
    if (ASDCP_FAILURE(result)) {
        return result.Value();
    }

    ASDCP::MXF::RGBAEssenceDescriptor* ed = 0;
    writer->OP1aHeader().GetMDObjectByType(dict->ul(ASDCP::MDD_RGBAEssenceDescriptor),
        reinterpret_cast<ASDCP::MXF::InterchangeObject**>(&ed));
    if (ed == 0) {
        return ASDCP::RESULT_FORMAT.Value();
    }
    apply_hdr_metadata(ed, hdr);
    return result.Value();
}

/* ---- JP2K Reader ---- */
asdcp_jp2k_reader_t asdcp_jp2k_reader_new(void) {
    Kumu::FileReaderFactory defaultFactory;
    return new ASDCP::JP2K::MXFReader(defaultFactory);
}

void asdcp_jp2k_reader_free(asdcp_jp2k_reader_t r) {
    delete static_cast<ASDCP::JP2K::MXFReader*>(r);
}

asdcp_result_t asdcp_jp2k_reader_open_read(asdcp_jp2k_reader_t r, const char* filename) {
    return static_cast<ASDCP::JP2K::MXFReader*>(r)->OpenRead(std::string(filename)).Value();
}

asdcp_result_t asdcp_jp2k_reader_close(asdcp_jp2k_reader_t r) {
    return static_cast<ASDCP::JP2K::MXFReader*>(r)->Close().Value();
}

asdcp_result_t asdcp_jp2k_reader_fill_picture_descriptor(asdcp_jp2k_reader_t r, asdcp_picture_descriptor_t* desc) {
    ASDCP::JP2K::PictureDescriptor pd;
    ASDCP::Result_t result = static_cast<ASDCP::JP2K::MXFReader*>(r)->FillPictureDescriptor(pd);
    if (ASDCP_SUCCESS(result.Value())) {
        cpp_to_c_picture_desc(pd, desc);
    }
    return result.Value();
}

asdcp_result_t asdcp_jp2k_reader_fill_writer_info(asdcp_jp2k_reader_t r, asdcp_writer_info_t* info) {
    ASDCP::WriterInfo wi;
    ASDCP::Result_t result = static_cast<ASDCP::JP2K::MXFReader*>(r)->FillWriterInfo(wi);
    if (ASDCP_SUCCESS(result.Value())) {
        cpp_to_c_writer_info(wi, info);
    }
    return result.Value();
}

asdcp_result_t asdcp_jp2k_reader_read_frame(asdcp_jp2k_reader_t r, uint32_t frame_number,
    uint8_t* buf, uint32_t buf_capacity, uint32_t* out_size,
    asdcp_aes_dec_context_t dec_ctx, asdcp_hmac_context_t hmac_ctx) {
    ASDCP::JP2K::FrameBuffer fb;
    fb.SetData(buf, buf_capacity);
    fb.Capacity(buf_capacity);
    ASDCP::Result_t result = static_cast<ASDCP::JP2K::MXFReader*>(r)->ReadFrame(
        frame_number, fb,
        static_cast<ASDCP::AESDecContext*>(dec_ctx),
        static_cast<ASDCP::HMACContext*>(hmac_ctx)
    );
    *out_size = fb.Size();
    return result.Value();
}

/* Read the TransferCharacteristic UL back off the RGBAEssenceDescriptor. */
asdcp_result_t asdcp_jp2k_reader_read_transfer_characteristic(asdcp_jp2k_reader_t r,
    uint8_t* out_ul, int32_t* has_transfer_characteristic) {
    const ASDCP::Dictionary* dict = &ASDCP::DefaultSMPTEDict();
    ASDCP::JP2K::MXFReader* reader = static_cast<ASDCP::JP2K::MXFReader*>(r);

    *has_transfer_characteristic = 0;
    ASDCP::MXF::InterchangeObject* obj = 0;
    reader->OP1aHeader().GetMDObjectByType(dict->ul(ASDCP::MDD_RGBAEssenceDescriptor), &obj);
    ASDCP::MXF::RGBAEssenceDescriptor* ed = dynamic_cast<ASDCP::MXF::RGBAEssenceDescriptor*>(obj);
    if (ed != 0 && !ed->TransferCharacteristic.empty()) {
        memcpy(out_ul, ed->TransferCharacteristic.const_get().Value(), 16);
        *has_transfer_characteristic = 1;
    }
    return ASDCP::RESULT_OK.Value();
}

/* Read all HDR/WCG metadata back off the RGBAEssenceDescriptor. */
asdcp_result_t asdcp_jp2k_reader_read_hdr(asdcp_jp2k_reader_t r, asdcp_hdr_metadata_t* hdr) {
    const ASDCP::Dictionary* dict = &ASDCP::DefaultSMPTEDict();
    ASDCP::JP2K::MXFReader* reader = static_cast<ASDCP::JP2K::MXFReader*>(r);

    memset(hdr, 0, sizeof(*hdr));
    ASDCP::MXF::InterchangeObject* obj = 0;
    reader->OP1aHeader().GetMDObjectByType(dict->ul(ASDCP::MDD_RGBAEssenceDescriptor), &obj);
    ASDCP::MXF::GenericPictureEssenceDescriptor* ed =
        dynamic_cast<ASDCP::MXF::GenericPictureEssenceDescriptor*>(obj);
    if (ed == 0) {
        return ASDCP::RESULT_FORMAT.Value();
    }
    read_hdr_metadata(ed, hdr);
    return ASDCP::RESULT_OK.Value();
}

/* ---- PCM Writer ---- */
asdcp_pcm_writer_t asdcp_pcm_writer_new(void) {
    return new ASDCP::PCM::MXFWriter();
}

void asdcp_pcm_writer_free(asdcp_pcm_writer_t w) {
    delete static_cast<ASDCP::PCM::MXFWriter*>(w);
}

asdcp_result_t asdcp_pcm_writer_open_write(asdcp_pcm_writer_t w, const char* filename,
    const asdcp_writer_info_t* info, const asdcp_audio_descriptor_t* desc, uint32_t header_size) {
    ASDCP::WriterInfo wi;
    c_to_cpp_writer_info(info, wi);
    ASDCP::PCM::AudioDescriptor ad;
    c_to_cpp_audio_desc(desc, ad);
    return static_cast<ASDCP::PCM::MXFWriter*>(w)->OpenWrite(std::string(filename), wi, ad, header_size).Value();
}

asdcp_result_t asdcp_pcm_writer_write_frame(asdcp_pcm_writer_t w,
    const uint8_t* frame_data, uint32_t frame_size,
    asdcp_aes_enc_context_t enc_ctx, asdcp_hmac_context_t hmac_ctx) {
    ASDCP::PCM::FrameBuffer fb;
    fb.SetData(const_cast<uint8_t*>(frame_data), frame_size);
    fb.Size(frame_size);
    return static_cast<ASDCP::PCM::MXFWriter*>(w)->WriteFrame(
        fb,
        static_cast<ASDCP::AESEncContext*>(enc_ctx),
        static_cast<ASDCP::HMACContext*>(hmac_ctx)
    ).Value();
}

asdcp_result_t asdcp_pcm_writer_finalize(asdcp_pcm_writer_t w) {
    return static_cast<ASDCP::PCM::MXFWriter*>(w)->Finalize().Value();
}

/* Open a PCM MXF and attach SMPTE 377-4 MCA label subdescriptors parsed from an
   asdcp-wrap style config string (e.g. "51(L,R,C,LFE,Ls,Rs),HI,VIN"). Mirrors
   asdcp-wrap.cpp: parse, OpenWrite, then add each subdescriptor to the header
   and link it from the WaveAudioDescriptor with the MCA ChannelAssignment. */
asdcp_result_t asdcp_pcm_writer_open_write_mca(asdcp_pcm_writer_t w, const char* filename,
    const asdcp_writer_info_t* info, const asdcp_audio_descriptor_t* desc,
    const char* mca_config, const char* mca_language, uint32_t header_size) {
    const ASDCP::Dictionary* dict = &ASDCP::DefaultSMPTEDict();
    ASDCP::PCM::MXFWriter* writer = static_cast<ASDCP::PCM::MXFWriter*>(w);

    ASDCP::WriterInfo wi;
    c_to_cpp_writer_info(info, wi);
    ASDCP::PCM::AudioDescriptor ad;
    c_to_cpp_audio_desc(desc, ad);

    ASDCP::MXF::ASDCP_MCAConfigParser mca(dict);
    const bool decoded = (mca_language == 0 || mca_language[0] == '\0')
        ? mca.DecodeString(std::string(mca_config))
        : mca.DecodeString(std::string(mca_config), std::string(mca_language));
    if (!decoded) {
        return ASDCP::RESULT_FORMAT.Value();
    }

    ASDCP::Result_t result = writer->OpenWrite(std::string(filename), wi, ad, header_size);
    if (ASDCP_FAILURE(result)) {
        return result.Value();
    }

    ASDCP::MXF::WaveAudioDescriptor* ed = 0;
    writer->OP1aHeader().GetMDObjectByType(dict->ul(ASDCP::MDD_WaveAudioDescriptor),
        reinterpret_cast<ASDCP::MXF::InterchangeObject**>(&ed));
    if (ed == 0) {
        return ASDCP::RESULT_FORMAT.Value();
    }
    if (mca.ChannelCount() != ed->ChannelCount) {
        return ASDCP::RESULT_FORMAT.Value();
    }

    ed->ChannelAssignment = ASDCP::UL(dict->ul(ASDCP::MDD_DCAudioChannelCfg_MCA));
    for (ASDCP::MXF::InterchangeObject_list_t::iterator i = mca.begin(); i != mca.end(); ++i) {
        writer->OP1aHeader().AddChildObject(*i);
        ed->SubDescriptors.push_back((*i)->InstanceUID);
        *i = 0; // header now owns it; stop the parser from freeing it too
    }
    return result.Value();
}

/* ---- PCM Reader ---- */
asdcp_pcm_reader_t asdcp_pcm_reader_new(void) {
    Kumu::FileReaderFactory defaultFactory;
    return new ASDCP::PCM::MXFReader(defaultFactory);
}

void asdcp_pcm_reader_free(asdcp_pcm_reader_t r) {
    delete static_cast<ASDCP::PCM::MXFReader*>(r);
}

asdcp_result_t asdcp_pcm_reader_open_read(asdcp_pcm_reader_t r, const char* filename) {
    return static_cast<ASDCP::PCM::MXFReader*>(r)->OpenRead(std::string(filename)).Value();
}

asdcp_result_t asdcp_pcm_reader_close(asdcp_pcm_reader_t r) {
    return static_cast<ASDCP::PCM::MXFReader*>(r)->Close().Value();
}

asdcp_result_t asdcp_pcm_reader_fill_audio_descriptor(asdcp_pcm_reader_t r, asdcp_audio_descriptor_t* desc) {
    ASDCP::PCM::AudioDescriptor ad;
    ASDCP::Result_t result = static_cast<ASDCP::PCM::MXFReader*>(r)->FillAudioDescriptor(ad);
    if (ASDCP_SUCCESS(result.Value())) {
        cpp_to_c_audio_desc(ad, desc);
    }
    return result.Value();
}

asdcp_result_t asdcp_pcm_reader_fill_writer_info(asdcp_pcm_reader_t r, asdcp_writer_info_t* info) {
    ASDCP::WriterInfo wi;
    ASDCP::Result_t result = static_cast<ASDCP::PCM::MXFReader*>(r)->FillWriterInfo(wi);
    if (ASDCP_SUCCESS(result.Value())) {
        cpp_to_c_writer_info(wi, info);
    }
    return result.Value();
}

asdcp_result_t asdcp_pcm_reader_read_frame(asdcp_pcm_reader_t r, uint32_t frame_number,
    uint8_t* buf, uint32_t buf_capacity, uint32_t* out_size,
    asdcp_aes_dec_context_t dec_ctx, asdcp_hmac_context_t hmac_ctx) {
    ASDCP::PCM::FrameBuffer fb;
    fb.SetData(buf, buf_capacity);
    fb.Capacity(buf_capacity);
    ASDCP::Result_t result = static_cast<ASDCP::PCM::MXFReader*>(r)->ReadFrame(
        frame_number, fb,
        static_cast<ASDCP::AESDecContext*>(dec_ctx),
        static_cast<ASDCP::HMACContext*>(hmac_ctx)
    );
    *out_size = fb.Size();
    return result.Value();
}

/* Count MCA label subdescriptors in a PCM MXF header and report whether the
   WaveAudioDescriptor carries the MCA ChannelAssignment UL. Proves labels
   written by asdcp_pcm_writer_open_write_mca survived a write/read cycle. */
asdcp_result_t asdcp_pcm_reader_read_mca_labels(asdcp_pcm_reader_t r,
    uint32_t* channel_label_count, uint32_t* soundfield_group_count,
    int32_t* has_mca_channel_assignment) {
    const ASDCP::Dictionary* dict = &ASDCP::DefaultSMPTEDict();
    ASDCP::PCM::MXFReader* reader = static_cast<ASDCP::PCM::MXFReader*>(r);

    std::list<ASDCP::MXF::InterchangeObject*> channels;
    reader->OP1aHeader().GetMDObjectsByType(
        dict->ul(ASDCP::MDD_AudioChannelLabelSubDescriptor), channels);
    *channel_label_count = static_cast<uint32_t>(channels.size());

    std::list<ASDCP::MXF::InterchangeObject*> groups;
    reader->OP1aHeader().GetMDObjectsByType(
        dict->ul(ASDCP::MDD_SoundfieldGroupLabelSubDescriptor), groups);
    *soundfield_group_count = static_cast<uint32_t>(groups.size());

    *has_mca_channel_assignment = 0;
    ASDCP::MXF::InterchangeObject* obj = 0;
    reader->OP1aHeader().GetMDObjectByType(dict->ul(ASDCP::MDD_WaveAudioDescriptor), &obj);
    ASDCP::MXF::WaveAudioDescriptor* wd = dynamic_cast<ASDCP::MXF::WaveAudioDescriptor*>(obj);
    if (wd != 0 && !wd->ChannelAssignment.empty()) {
        ASDCP::UL mca_ul(dict->ul(ASDCP::MDD_DCAudioChannelCfg_MCA));
        if (wd->ChannelAssignment == mca_ul) {
            *has_mca_channel_assignment = 1;
        }
    }
    return ASDCP::RESULT_OK.Value();
}

/* 0 = audio channel, 1 = soundfield group, 2 = group of soundfield groups,
   -1 = not an MCA label subdescriptor. */
static int32_t mca_label_kind(ASDCP::MXF::InterchangeObject* obj) {
    if (dynamic_cast<ASDCP::MXF::AudioChannelLabelSubDescriptor*>(obj) != 0) {
        return 0;
    }
    if (dynamic_cast<ASDCP::MXF::SoundfieldGroupLabelSubDescriptor*>(obj) != 0) {
        return 1;
    }
    if (dynamic_cast<ASDCP::MXF::GroupOfSoundfieldGroupsLabelSubDescriptor*>(obj) != 0) {
        return 2;
    }
    return -1;
}

/* The WaveAudioDescriptor's SubDescriptors array keeps the order the labels
   were written in, which the header's packet list does not promise. */
static void collect_mca_labels(ASDCP::MXF::OP1aHeader& header,
    std::list<ASDCP::MXF::InterchangeObject*>& labels) {
    const ASDCP::Dictionary* dict = &ASDCP::DefaultSMPTEDict();
    ASDCP::MXF::InterchangeObject* obj = 0;
    header.GetMDObjectByType(dict->ul(ASDCP::MDD_WaveAudioDescriptor), &obj);
    ASDCP::MXF::WaveAudioDescriptor* wd = dynamic_cast<ASDCP::MXF::WaveAudioDescriptor*>(obj);
    if (wd == 0) {
        return;
    }
    for (size_t k = 0; k < wd->SubDescriptors.size(); k++) {
        ASDCP::MXF::InterchangeObject* sub = 0;
        ASDCP::Result_t result = header.GetMDObjectByID(wd->SubDescriptors[k], &sub);
        if (ASDCP_FAILURE(result)) {
            continue;
        }
        if (mca_label_kind(sub) >= 0) {
            labels.push_back(sub);
        }
    }
}

static void copy_mca_string(const std::string& src, char* dst) {
    size_t length = src.size();
    if (length > ASDCP_MCA_STRING_CAPACITY - 1) {
        length = ASDCP_MCA_STRING_CAPACITY - 1;
    }
    memcpy(dst, src.data(), length);
    dst[length] = 0;
}

asdcp_result_t asdcp_pcm_reader_mca_label_count(asdcp_pcm_reader_t r, uint32_t* out_count) {
    std::list<ASDCP::MXF::InterchangeObject*> labels;
    collect_mca_labels(static_cast<ASDCP::PCM::MXFReader*>(r)->OP1aHeader(), labels);
    *out_count = static_cast<uint32_t>(labels.size());
    return ASDCP::RESULT_OK.Value();
}

static asdcp_result_t fill_mca_label(const std::list<ASDCP::MXF::InterchangeObject*>& labels,
    uint32_t index, asdcp_mca_label_t* out_label) {
    if (index >= labels.size()) {
        return ASDCP::RESULT_RANGE.Value();
    }
    std::list<ASDCP::MXF::InterchangeObject*>::const_iterator li = labels.begin();
    for (uint32_t k = 0; k < index; k++) {
        ++li;
    }
    ASDCP::MXF::MCALabelSubDescriptor* label =
        dynamic_cast<ASDCP::MXF::MCALabelSubDescriptor*>(*li);

    memset(out_label, 0, sizeof(*out_label));
    out_label->kind = mca_label_kind(*li);
    copy_mca_string(label->MCATagSymbol, out_label->tag_symbol);
    memcpy(out_label->label_dictionary_id, label->MCALabelDictionaryID.Value(), 16);
    memcpy(out_label->link_id, label->MCALinkID.Value(), 16);
    if (!label->MCATagName.empty()) {
        out_label->has_tag_name = 1;
        copy_mca_string(label->MCATagName.const_get(), out_label->tag_name);
    }
    if (!label->MCAChannelID.empty()) {
        out_label->has_channel_id = 1;
        out_label->channel_id = label->MCAChannelID.const_get();
    }
    if (!label->RFC5646SpokenLanguage.empty()) {
        out_label->has_spoken_language = 1;
        copy_mca_string(label->RFC5646SpokenLanguage.const_get(), out_label->spoken_language);
    }
    ASDCP::MXF::AudioChannelLabelSubDescriptor* channel =
        dynamic_cast<ASDCP::MXF::AudioChannelLabelSubDescriptor*>(*li);
    if (channel != 0 && !channel->SoundfieldGroupLinkID.empty()) {
        out_label->has_soundfield_group_link_id = 1;
        memcpy(out_label->soundfield_group_link_id,
            channel->SoundfieldGroupLinkID.const_get().Value(), 16);
    }
    return ASDCP::RESULT_OK.Value();
}

asdcp_result_t asdcp_pcm_reader_mca_label_info(asdcp_pcm_reader_t r, uint32_t index,
    asdcp_mca_label_t* out_label) {
    std::list<ASDCP::MXF::InterchangeObject*> labels;
    collect_mca_labels(static_cast<ASDCP::PCM::MXFReader*>(r)->OP1aHeader(), labels);
    return fill_mca_label(labels, index, out_label);
}

/* ---- TimedText Writer ---- */
asdcp_timed_text_writer_t asdcp_timed_text_writer_new(void) {
    return new ASDCP::TimedText::MXFWriter();
}

void asdcp_timed_text_writer_free(asdcp_timed_text_writer_t w) {
    delete static_cast<ASDCP::TimedText::MXFWriter*>(w);
}

asdcp_result_t asdcp_timed_text_writer_open_write(asdcp_timed_text_writer_t w, const char* filename,
    const asdcp_writer_info_t* info, const asdcp_timed_text_descriptor_t* desc, uint32_t header_size) {
    ASDCP::WriterInfo wi;
    c_to_cpp_writer_info(info, wi);
    ASDCP::TimedText::TimedTextDescriptor td;
    c_to_cpp_timed_text_desc(desc, td);
    return static_cast<ASDCP::TimedText::MXFWriter*>(w)->OpenWrite(std::string(filename), wi, td, header_size).Value();
}

asdcp_result_t asdcp_timed_text_writer_write_timed_text_resource(asdcp_timed_text_writer_t w,
    const char* xml_doc,
    asdcp_aes_enc_context_t enc_ctx, asdcp_hmac_context_t hmac_ctx) {
    std::string doc(xml_doc);
    return static_cast<ASDCP::TimedText::MXFWriter*>(w)->WriteTimedTextResource(
        doc,
        static_cast<ASDCP::AESEncContext*>(enc_ctx),
        static_cast<ASDCP::HMACContext*>(hmac_ctx)
    ).Value();
}

asdcp_result_t asdcp_timed_text_writer_write_ancillary_resource(asdcp_timed_text_writer_t w,
    const uint8_t* resource_data, uint32_t resource_size,
    const uint8_t* resource_uuid, const char* mime_type,
    asdcp_aes_enc_context_t enc_ctx, asdcp_hmac_context_t hmac_ctx) {
    ASDCP::TimedText::FrameBuffer fb;
    fb.SetData(const_cast<uint8_t*>(resource_data), resource_size);
    fb.Size(resource_size);
    fb.AssetID(resource_uuid);
    fb.MIMEType(std::string(mime_type));
    return static_cast<ASDCP::TimedText::MXFWriter*>(w)->WriteAncillaryResource(
        fb,
        static_cast<ASDCP::AESEncContext*>(enc_ctx),
        static_cast<ASDCP::HMACContext*>(hmac_ctx)
    ).Value();
}

asdcp_result_t asdcp_timed_text_writer_finalize(asdcp_timed_text_writer_t w) {
    return static_cast<ASDCP::TimedText::MXFWriter*>(w)->Finalize().Value();
}

/* The writer records ancillary resources (their UUID + MIME type) from the
   descriptor's ResourceList at open time; without them the reader cannot
   enumerate the resources. Build that list from the parallel arrays. */
asdcp_result_t asdcp_timed_text_writer_open_write_with_resources(asdcp_timed_text_writer_t w,
    const char* filename, const asdcp_writer_info_t* info, const asdcp_timed_text_descriptor_t* desc,
    const uint8_t* resource_uuids, const int32_t* resource_types, uint32_t resource_count,
    uint32_t header_size) {
    ASDCP::WriterInfo wi;
    c_to_cpp_writer_info(info, wi);
    ASDCP::TimedText::TimedTextDescriptor td;
    c_to_cpp_timed_text_desc(desc, td);
    for (uint32_t i = 0; i < resource_count; i++) {
        ASDCP::TimedText::TimedTextResourceDescriptor rd;
        memcpy(rd.ResourceID, resource_uuids + i * 16, 16);
        rd.Type = static_cast<ASDCP::TimedText::MIMEType_t>(resource_types[i]);
        td.ResourceList.push_back(rd);
    }
    return static_cast<ASDCP::TimedText::MXFWriter*>(w)->OpenWrite(std::string(filename), wi, td, header_size).Value();
}

/* ---- TimedText Reader ---- */
asdcp_timed_text_reader_t asdcp_timed_text_reader_new(void) {
    Kumu::FileReaderFactory defaultFactory;
    return new ASDCP::TimedText::MXFReader(defaultFactory);
}

void asdcp_timed_text_reader_free(asdcp_timed_text_reader_t r) {
    delete static_cast<ASDCP::TimedText::MXFReader*>(r);
}

asdcp_result_t asdcp_timed_text_reader_open_read(asdcp_timed_text_reader_t r, const char* filename) {
    return static_cast<ASDCP::TimedText::MXFReader*>(r)->OpenRead(std::string(filename)).Value();
}

asdcp_result_t asdcp_timed_text_reader_close(asdcp_timed_text_reader_t r) {
    return static_cast<ASDCP::TimedText::MXFReader*>(r)->Close().Value();
}

asdcp_result_t asdcp_timed_text_reader_fill_descriptor(asdcp_timed_text_reader_t r, asdcp_timed_text_descriptor_t* desc) {
    ASDCP::TimedText::TimedTextDescriptor td;
    ASDCP::Result_t result = static_cast<ASDCP::TimedText::MXFReader*>(r)->FillTimedTextDescriptor(td);
    if (ASDCP_SUCCESS(result.Value())) {
        cpp_to_c_timed_text_desc(td, desc);
    }
    return result.Value();
}

asdcp_result_t asdcp_timed_text_reader_fill_writer_info(asdcp_timed_text_reader_t r, asdcp_writer_info_t* info) {
    ASDCP::WriterInfo wi;
    ASDCP::Result_t result = static_cast<ASDCP::TimedText::MXFReader*>(r)->FillWriterInfo(wi);
    if (ASDCP_SUCCESS(result.Value())) {
        cpp_to_c_writer_info(wi, info);
    }
    return result.Value();
}

asdcp_result_t asdcp_timed_text_reader_read_timed_text_resource(asdcp_timed_text_reader_t r,
    uint8_t* buf, uint32_t buf_capacity, uint32_t* out_size,
    asdcp_aes_dec_context_t dec_ctx, asdcp_hmac_context_t hmac_ctx) {
    std::string doc;
    ASDCP::Result_t result = static_cast<ASDCP::TimedText::MXFReader*>(r)->ReadTimedTextResource(
        doc,
        static_cast<ASDCP::AESDecContext*>(dec_ctx),
        static_cast<ASDCP::HMACContext*>(hmac_ctx)
    );
    if (ASDCP_SUCCESS(result.Value())) {
        uint32_t doc_len = static_cast<uint32_t>(doc.size());
        // report the size the caller needs, even when we can't deliver the bytes
        *out_size = doc_len;
        if (doc_len > buf_capacity) {
            return ASDCP::RESULT_SMALLBUF.Value();
        }
        memcpy(buf, doc.data(), doc_len);
    } else {
        *out_size = 0;
    }
    return result.Value();
}

asdcp_result_t asdcp_timed_text_reader_ancillary_resource_count(asdcp_timed_text_reader_t r,
    uint32_t* out_count) {
    ASDCP::TimedText::TimedTextDescriptor td;
    ASDCP::Result_t result = static_cast<ASDCP::TimedText::MXFReader*>(r)->FillTimedTextDescriptor(td);
    if (ASDCP_SUCCESS(result.Value())) {
        *out_count = static_cast<uint32_t>(td.ResourceList.size());
    }
    return result.Value();
}

asdcp_result_t asdcp_timed_text_reader_ancillary_resource_info(asdcp_timed_text_reader_t r,
    uint32_t index, uint8_t* out_uuid, int32_t* out_type) {
    ASDCP::TimedText::TimedTextDescriptor td;
    ASDCP::Result_t result = static_cast<ASDCP::TimedText::MXFReader*>(r)->FillTimedTextDescriptor(td);
    if (! ASDCP_SUCCESS(result.Value())) {
        return result.Value();
    }
    if (index >= td.ResourceList.size()) {
        return ASDCP::RESULT_RANGE.Value();
    }
    ASDCP::TimedText::ResourceList_t::const_iterator ri = td.ResourceList.begin();
    for (uint32_t k = 0; k < index; k++) {
        ++ri;
    }
    memcpy(out_uuid, ri->ResourceID, 16);
    *out_type = static_cast<int32_t>(ri->Type);
    return ASDCP::RESULT_OK.Value();
}

asdcp_result_t asdcp_timed_text_reader_read_ancillary_resource(asdcp_timed_text_reader_t r,
    const uint8_t* resource_uuid, uint8_t* buf, uint32_t buf_capacity, uint32_t* out_size,
    asdcp_aes_dec_context_t dec_ctx, asdcp_hmac_context_t hmac_ctx) {
    // The reader sizes the FrameBuffer to the stream payload, so it must own its
    // memory (SetData'd buffers can't be resized). Read into it, then copy out.
    ASDCP::TimedText::FrameBuffer fb;
    fb.Capacity(buf_capacity);
    ASDCP::Result_t result = static_cast<ASDCP::TimedText::MXFReader*>(r)->ReadAncillaryResource(
        resource_uuid, fb,
        static_cast<ASDCP::AESDecContext*>(dec_ctx),
        static_cast<ASDCP::HMACContext*>(hmac_ctx)
    );
    if (! ASDCP_SUCCESS(result.Value())) {
        *out_size = 0;
        return result.Value();
    }
    // report the size the caller needs, even when the buffer is too small
    *out_size = fb.Size();
    if (fb.Size() > buf_capacity) {
        return ASDCP::RESULT_SMALLBUF.Value();
    }
    memcpy(buf, fb.RoData(), fb.Size());
    return result.Value();
}

/* ---- Atmos Writer ---- */
asdcp_atmos_writer_t asdcp_atmos_writer_new(void) {
    return new ASDCP::ATMOS::MXFWriter();
}

void asdcp_atmos_writer_free(asdcp_atmos_writer_t w) {
    delete static_cast<ASDCP::ATMOS::MXFWriter*>(w);
}

asdcp_result_t asdcp_atmos_writer_open_write(asdcp_atmos_writer_t w, const char* filename,
    const asdcp_writer_info_t* info, const asdcp_atmos_descriptor_t* desc, uint32_t header_size) {
    ASDCP::WriterInfo wi;
    c_to_cpp_writer_info(info, wi);
    ASDCP::ATMOS::AtmosDescriptor ad;
    c_to_cpp_atmos_desc(desc, ad);
    return static_cast<ASDCP::ATMOS::MXFWriter*>(w)->OpenWrite(std::string(filename), wi, ad, header_size).Value();
}

asdcp_result_t asdcp_atmos_writer_write_frame(asdcp_atmos_writer_t w,
    const uint8_t* frame_data, uint32_t frame_size,
    asdcp_aes_enc_context_t enc_ctx, asdcp_hmac_context_t hmac_ctx) {
    ASDCP::DCData::FrameBuffer fb;
    fb.SetData(const_cast<uint8_t*>(frame_data), frame_size);
    fb.Size(frame_size);
    return static_cast<ASDCP::ATMOS::MXFWriter*>(w)->WriteFrame(
        fb,
        static_cast<ASDCP::AESEncContext*>(enc_ctx),
        static_cast<ASDCP::HMACContext*>(hmac_ctx)
    ).Value();
}

asdcp_result_t asdcp_atmos_writer_finalize(asdcp_atmos_writer_t w) {
    return static_cast<ASDCP::ATMOS::MXFWriter*>(w)->Finalize().Value();
}

/* ---- Atmos Reader ---- */
asdcp_atmos_reader_t asdcp_atmos_reader_new(void) {
    Kumu::FileReaderFactory defaultFactory;
    return new ASDCP::ATMOS::MXFReader(defaultFactory);
}

void asdcp_atmos_reader_free(asdcp_atmos_reader_t r) {
    delete static_cast<ASDCP::ATMOS::MXFReader*>(r);
}

asdcp_result_t asdcp_atmos_reader_open_read(asdcp_atmos_reader_t r, const char* filename) {
    return static_cast<ASDCP::ATMOS::MXFReader*>(r)->OpenRead(std::string(filename)).Value();
}

asdcp_result_t asdcp_atmos_reader_close(asdcp_atmos_reader_t r) {
    return static_cast<ASDCP::ATMOS::MXFReader*>(r)->Close().Value();
}

asdcp_result_t asdcp_atmos_reader_fill_atmos_descriptor(asdcp_atmos_reader_t r, asdcp_atmos_descriptor_t* desc) {
    ASDCP::ATMOS::AtmosDescriptor ad;
    ASDCP::Result_t result = static_cast<ASDCP::ATMOS::MXFReader*>(r)->FillAtmosDescriptor(ad);
    if (ASDCP_SUCCESS(result.Value())) {
        cpp_to_c_atmos_desc(ad, desc);
    }
    return result.Value();
}

asdcp_result_t asdcp_atmos_reader_fill_writer_info(asdcp_atmos_reader_t r, asdcp_writer_info_t* info) {
    ASDCP::WriterInfo wi;
    ASDCP::Result_t result = static_cast<ASDCP::ATMOS::MXFReader*>(r)->FillWriterInfo(wi);
    if (ASDCP_SUCCESS(result.Value())) {
        cpp_to_c_writer_info(wi, info);
    }
    return result.Value();
}

asdcp_result_t asdcp_atmos_reader_read_frame(asdcp_atmos_reader_t r, uint32_t frame_number,
    uint8_t* buf, uint32_t buf_capacity, uint32_t* out_size,
    asdcp_aes_dec_context_t dec_ctx, asdcp_hmac_context_t hmac_ctx) {
    ASDCP::DCData::FrameBuffer fb;
    fb.SetData(buf, buf_capacity);
    fb.Capacity(buf_capacity);
    ASDCP::Result_t result = static_cast<ASDCP::ATMOS::MXFReader*>(r)->ReadFrame(
        frame_number, fb,
        static_cast<ASDCP::AESDecContext*>(dec_ctx),
        static_cast<ASDCP::HMACContext*>(hmac_ctx)
    );
    *out_size = fb.Size();
    return result.Value();
}

/* ---- JP2K Stereoscopic Writer ---- */
asdcp_jp2k_s_writer_t asdcp_jp2k_s_writer_new(void) {
    return new ASDCP::JP2K::MXFSWriter();
}

void asdcp_jp2k_s_writer_free(asdcp_jp2k_s_writer_t w) {
    delete static_cast<ASDCP::JP2K::MXFSWriter*>(w);
}

asdcp_result_t asdcp_jp2k_s_writer_open_write(asdcp_jp2k_s_writer_t w, const char* filename,
    const asdcp_writer_info_t* info, const asdcp_picture_descriptor_t* desc, uint32_t header_size) {
    ASDCP::WriterInfo wi;
    c_to_cpp_writer_info(info, wi);
    ASDCP::JP2K::PictureDescriptor pd;
    c_to_cpp_picture_desc(desc, pd);
    return static_cast<ASDCP::JP2K::MXFSWriter*>(w)->OpenWrite(std::string(filename), wi, pd, header_size).Value();
}

asdcp_result_t asdcp_jp2k_s_writer_write_frame(asdcp_jp2k_s_writer_t w,
    const uint8_t* frame_data, uint32_t frame_size, int32_t phase,
    asdcp_aes_enc_context_t enc_ctx, asdcp_hmac_context_t hmac_ctx) {
    ASDCP::JP2K::FrameBuffer fb;
    fb.SetData(const_cast<uint8_t*>(frame_data), frame_size);
    fb.Size(frame_size);
    return static_cast<ASDCP::JP2K::MXFSWriter*>(w)->WriteFrame(
        fb,
        static_cast<ASDCP::JP2K::StereoscopicPhase_t>(phase),
        static_cast<ASDCP::AESEncContext*>(enc_ctx),
        static_cast<ASDCP::HMACContext*>(hmac_ctx)
    ).Value();
}

asdcp_result_t asdcp_jp2k_s_writer_finalize(asdcp_jp2k_s_writer_t w) {
    return static_cast<ASDCP::JP2K::MXFSWriter*>(w)->Finalize().Value();
}

/* ---- JP2K Stereoscopic Reader ---- */
asdcp_jp2k_s_reader_t asdcp_jp2k_s_reader_new(void) {
    Kumu::FileReaderFactory defaultFactory;
    return new ASDCP::JP2K::MXFSReader(defaultFactory);
}

void asdcp_jp2k_s_reader_free(asdcp_jp2k_s_reader_t r) {
    delete static_cast<ASDCP::JP2K::MXFSReader*>(r);
}

asdcp_result_t asdcp_jp2k_s_reader_open_read(asdcp_jp2k_s_reader_t r, const char* filename) {
    return static_cast<ASDCP::JP2K::MXFSReader*>(r)->OpenRead(std::string(filename)).Value();
}

asdcp_result_t asdcp_jp2k_s_reader_close(asdcp_jp2k_s_reader_t r) {
    return static_cast<ASDCP::JP2K::MXFSReader*>(r)->Close().Value();
}

asdcp_result_t asdcp_jp2k_s_reader_fill_picture_descriptor(asdcp_jp2k_s_reader_t r, asdcp_picture_descriptor_t* desc) {
    ASDCP::JP2K::PictureDescriptor pd;
    ASDCP::Result_t result = static_cast<ASDCP::JP2K::MXFSReader*>(r)->FillPictureDescriptor(pd);
    if (ASDCP_SUCCESS(result.Value())) {
        cpp_to_c_picture_desc(pd, desc);
    }
    return result.Value();
}

asdcp_result_t asdcp_jp2k_s_reader_fill_writer_info(asdcp_jp2k_s_reader_t r, asdcp_writer_info_t* info) {
    ASDCP::WriterInfo wi;
    ASDCP::Result_t result = static_cast<ASDCP::JP2K::MXFSReader*>(r)->FillWriterInfo(wi);
    if (ASDCP_SUCCESS(result.Value())) {
        cpp_to_c_writer_info(wi, info);
    }
    return result.Value();
}

asdcp_result_t asdcp_jp2k_s_reader_read_frame(asdcp_jp2k_s_reader_t r, uint32_t frame_number,
    int32_t phase, uint8_t* buf, uint32_t buf_capacity, uint32_t* out_size,
    asdcp_aes_dec_context_t dec_ctx, asdcp_hmac_context_t hmac_ctx) {
    ASDCP::JP2K::FrameBuffer fb;
    fb.SetData(buf, buf_capacity);
    fb.Capacity(buf_capacity);
    ASDCP::Result_t result = static_cast<ASDCP::JP2K::MXFSReader*>(r)->ReadFrame(
        frame_number,
        static_cast<ASDCP::JP2K::StereoscopicPhase_t>(phase),
        fb,
        static_cast<ASDCP::AESDecContext*>(dec_ctx),
        static_cast<ASDCP::HMACContext*>(hmac_ctx)
    );
    *out_size = fb.Size();
    return result.Value();
}

/* ================= AS-02 (IMF / ST 2067-5) ================= */

/* ---- AS-02 JP2K Writer ---- */
asdcp_as02_jp2k_writer_t asdcp_as02_jp2k_writer_new(void) {
    return new AS_02::JP2K::MXFWriter();
}

void asdcp_as02_jp2k_writer_free(asdcp_as02_jp2k_writer_t w) {
    delete static_cast<AS_02::JP2K::MXFWriter*>(w);
}

/* Rsiz profile bits, ISO/IEC 15444-1 Amd 8. The low 12 bits carry the profile;
   within them the IMF profiles use 0x0f00 for the family (0x0400 2K, 0x0500 4K,
   0x0600 8K), 0x00f0 for the sub level and 0x000f for the main level. Sub level
   0 means the profile is reversible. */
static const uint16_t RSIZ_PROFILE_MASK = 0x0fff;
static const uint16_t RSIZ_IMF_FAMILY_MASK = 0x0f00;
static const uint16_t RSIZ_IMF_SUB_LEVEL_MASK = 0x00f0;
static const uint16_t RSIZ_PROFILE_CINEMA_2K = 0x0003;
static const uint16_t RSIZ_PROFILE_CINEMA_4K = 0x0004;
static const uint16_t RSIZ_IMF_FAMILY_2K = 0x0400;
static const uint16_t RSIZ_IMF_FAMILY_4K = 0x0500;
static const uint16_t RSIZ_IMF_FAMILY_8K = 0x0600;

/* Ssize packs the bit depth minus one in its low 7 bits and the signed flag in
   the top bit. */
static const uint8_t SSIZE_DEPTH_MASK = 0x7f;

static const uint16_t RSIZ_IMF_MAIN_LEVEL_MASK = 0x000f;
static const uint16_t RSIZ_IMF_SUB_LEVEL_SHIFT = 4;

/* Every level-specific IMF picture essence coding label MDD.cpp names, keyed by
   the profile family, main level and sub level its Rsiz encodes. Sub level 0 is
   a reversible profile. */
struct imf_essence_coding_entry {
    uint16_t family;
    uint8_t main_level;
    uint8_t sub_level;
    ASDCP::MDD_t label;
};

static const imf_essence_coding_entry IMF_ESSENCE_CODINGS[] = {
    { RSIZ_IMF_FAMILY_2K,  1, 1, ASDCP::MDD_JP2KEssenceCompression_IMFProfile_2K_Lossy_1_1 },
    { RSIZ_IMF_FAMILY_2K,  2, 1, ASDCP::MDD_JP2KEssenceCompression_IMFProfile_2K_Lossy_2_1 },
    { RSIZ_IMF_FAMILY_2K,  3, 1, ASDCP::MDD_JP2KEssenceCompression_IMFProfile_2K_Lossy_3_1 },
    { RSIZ_IMF_FAMILY_2K,  4, 1, ASDCP::MDD_JP2KEssenceCompression_IMFProfile_2K_Lossy_4_1 },
    { RSIZ_IMF_FAMILY_2K,  4, 2, ASDCP::MDD_JP2KEssenceCompression_IMFProfile_2K_Lossy_4_2 },
    { RSIZ_IMF_FAMILY_2K,  5, 1, ASDCP::MDD_JP2KEssenceCompression_IMFProfile_2K_Lossy_5_1 },
    { RSIZ_IMF_FAMILY_2K,  5, 2, ASDCP::MDD_JP2KEssenceCompression_IMFProfile_2K_Lossy_5_2 },
    { RSIZ_IMF_FAMILY_2K,  5, 3, ASDCP::MDD_JP2KEssenceCompression_IMFProfile_2K_Lossy_5_3 },
    { RSIZ_IMF_FAMILY_2K,  6, 1, ASDCP::MDD_JP2KEssenceCompression_IMFProfile_2K_Lossy_6_1 },
    { RSIZ_IMF_FAMILY_2K,  6, 2, ASDCP::MDD_JP2KEssenceCompression_IMFProfile_2K_Lossy_6_2 },
    { RSIZ_IMF_FAMILY_2K,  6, 3, ASDCP::MDD_JP2KEssenceCompression_IMFProfile_2K_Lossy_6_3 },
    { RSIZ_IMF_FAMILY_2K,  6, 4, ASDCP::MDD_JP2KEssenceCompression_IMFProfile_2K_Lossy_6_4 },
    { RSIZ_IMF_FAMILY_4K,  1, 1, ASDCP::MDD_JP2KEssenceCompression_IMFProfile_4K_Lossy_1_1 },
    { RSIZ_IMF_FAMILY_4K,  2, 1, ASDCP::MDD_JP2KEssenceCompression_IMFProfile_4K_Lossy_2_1 },
    { RSIZ_IMF_FAMILY_4K,  3, 1, ASDCP::MDD_JP2KEssenceCompression_IMFProfile_4K_Lossy_3_1 },
    { RSIZ_IMF_FAMILY_4K,  4, 1, ASDCP::MDD_JP2KEssenceCompression_IMFProfile_4K_Lossy_4_1 },
    { RSIZ_IMF_FAMILY_4K,  4, 2, ASDCP::MDD_JP2KEssenceCompression_IMFProfile_4K_Lossy_4_2 },
    { RSIZ_IMF_FAMILY_4K,  5, 1, ASDCP::MDD_JP2KEssenceCompression_IMFProfile_4K_Lossy_5_1 },
    { RSIZ_IMF_FAMILY_4K,  5, 2, ASDCP::MDD_JP2KEssenceCompression_IMFProfile_4K_Lossy_5_2 },
    { RSIZ_IMF_FAMILY_4K,  5, 3, ASDCP::MDD_JP2KEssenceCompression_IMFProfile_4K_Lossy_5_3 },
    { RSIZ_IMF_FAMILY_4K,  6, 1, ASDCP::MDD_JP2KEssenceCompression_IMFProfile_4K_Lossy_6_1 },
    { RSIZ_IMF_FAMILY_4K,  6, 2, ASDCP::MDD_JP2KEssenceCompression_IMFProfile_4K_Lossy_6_2 },
    { RSIZ_IMF_FAMILY_4K,  6, 3, ASDCP::MDD_JP2KEssenceCompression_IMFProfile_4K_Lossy_6_3 },
    { RSIZ_IMF_FAMILY_4K,  6, 4, ASDCP::MDD_JP2KEssenceCompression_IMFProfile_4K_Lossy_6_4 },
    { RSIZ_IMF_FAMILY_4K,  7, 1, ASDCP::MDD_JP2KEssenceCompression_IMFProfile_4K_Lossy_7_1 },
    { RSIZ_IMF_FAMILY_4K,  7, 2, ASDCP::MDD_JP2KEssenceCompression_IMFProfile_4K_Lossy_7_2 },
    { RSIZ_IMF_FAMILY_4K,  7, 3, ASDCP::MDD_JP2KEssenceCompression_IMFProfile_4K_Lossy_7_3 },
    { RSIZ_IMF_FAMILY_4K,  7, 4, ASDCP::MDD_JP2KEssenceCompression_IMFProfile_4K_Lossy_7_4 },
    { RSIZ_IMF_FAMILY_4K,  7, 5, ASDCP::MDD_JP2KEssenceCompression_IMFProfile_4K_Lossy_7_5 },
    { RSIZ_IMF_FAMILY_4K,  8, 1, ASDCP::MDD_JP2KEssenceCompression_IMFProfile_4K_Lossy_8_1 },
    { RSIZ_IMF_FAMILY_4K,  8, 2, ASDCP::MDD_JP2KEssenceCompression_IMFProfile_4K_Lossy_8_2 },
    { RSIZ_IMF_FAMILY_4K,  8, 3, ASDCP::MDD_JP2KEssenceCompression_IMFProfile_4K_Lossy_8_3 },
    { RSIZ_IMF_FAMILY_4K,  8, 4, ASDCP::MDD_JP2KEssenceCompression_IMFProfile_4K_Lossy_8_4 },
    { RSIZ_IMF_FAMILY_4K,  8, 5, ASDCP::MDD_JP2KEssenceCompression_IMFProfile_4K_Lossy_8_5 },
    { RSIZ_IMF_FAMILY_4K,  8, 6, ASDCP::MDD_JP2KEssenceCompression_IMFProfile_4K_Lossy_8_6 },
    { RSIZ_IMF_FAMILY_2K,  1, 0, ASDCP::MDD_JP2KEssenceCompression_IMFProfile_2K_Reversible_1_0 },
    { RSIZ_IMF_FAMILY_2K,  2, 0, ASDCP::MDD_JP2KEssenceCompression_IMFProfile_2K_Reversible_2_0 },
    { RSIZ_IMF_FAMILY_2K,  3, 0, ASDCP::MDD_JP2KEssenceCompression_IMFProfile_2K_Reversible_3_0 },
    { RSIZ_IMF_FAMILY_2K,  4, 0, ASDCP::MDD_JP2KEssenceCompression_IMFProfile_2K_Reversible_4_0 },
    { RSIZ_IMF_FAMILY_2K,  5, 0, ASDCP::MDD_JP2KEssenceCompression_IMFProfile_2K_Reversible_5_0 },
    { RSIZ_IMF_FAMILY_2K,  6, 0, ASDCP::MDD_JP2KEssenceCompression_IMFProfile_2K_Reversible_6_0 },
    { RSIZ_IMF_FAMILY_4K,  1, 0, ASDCP::MDD_JP2KEssenceCompression_IMFProfile_4K_Reversible_1_0 },
    { RSIZ_IMF_FAMILY_4K,  2, 0, ASDCP::MDD_JP2KEssenceCompression_IMFProfile_4K_Reversible_2_0 },
    { RSIZ_IMF_FAMILY_4K,  3, 0, ASDCP::MDD_JP2KEssenceCompression_IMFProfile_4K_Reversible_3_0 },
    { RSIZ_IMF_FAMILY_4K,  4, 0, ASDCP::MDD_JP2KEssenceCompression_IMFProfile_4K_Reversible_4_0 },
    { RSIZ_IMF_FAMILY_4K,  5, 0, ASDCP::MDD_JP2KEssenceCompression_IMFProfile_4K_Reversible_5_0 },
    { RSIZ_IMF_FAMILY_4K,  6, 0, ASDCP::MDD_JP2KEssenceCompression_IMFProfile_4K_Reversible_6_0 },
    { RSIZ_IMF_FAMILY_4K,  7, 0, ASDCP::MDD_JP2KEssenceCompression_IMFProfile_4K_Reversible_7_0 },
    { RSIZ_IMF_FAMILY_4K,  8, 0, ASDCP::MDD_JP2KEssenceCompression_IMFProfile_4K_Reversible_8_0 },
    { RSIZ_IMF_FAMILY_8K,  1, 0, ASDCP::MDD_JP2KEssenceCompression_IMFProfile_8K_Reversible_1_0 },
    { RSIZ_IMF_FAMILY_8K,  2, 0, ASDCP::MDD_JP2KEssenceCompression_IMFProfile_8K_Reversible_2_0 },
    { RSIZ_IMF_FAMILY_8K,  3, 0, ASDCP::MDD_JP2KEssenceCompression_IMFProfile_8K_Reversible_3_0 },
    { RSIZ_IMF_FAMILY_8K,  4, 0, ASDCP::MDD_JP2KEssenceCompression_IMFProfile_8K_Reversible_4_0 },
    { RSIZ_IMF_FAMILY_8K,  5, 0, ASDCP::MDD_JP2KEssenceCompression_IMFProfile_8K_Reversible_5_0 },
    { RSIZ_IMF_FAMILY_8K,  6, 0, ASDCP::MDD_JP2KEssenceCompression_IMFProfile_8K_Reversible_6_0 },
    { RSIZ_IMF_FAMILY_8K,  7, 0, ASDCP::MDD_JP2KEssenceCompression_IMFProfile_8K_Reversible_7_0 },
    { RSIZ_IMF_FAMILY_8K,  8, 0, ASDCP::MDD_JP2KEssenceCompression_IMFProfile_8K_Reversible_8_0 },
    { RSIZ_IMF_FAMILY_8K,  9, 0, ASDCP::MDD_JP2KEssenceCompression_IMFProfile_8K_Reversible_9_0 },
    { RSIZ_IMF_FAMILY_8K, 10, 0, ASDCP::MDD_JP2KEssenceCompression_IMFProfile_8K_Reversible_10_0 },
};

/* The PictureEssenceCoding label a codestream's Rsiz calls for. */
static ASDCP::MDD_t essence_coding_for_rsize(uint16_t rsize) {
    const uint16_t profile = rsize & RSIZ_PROFILE_MASK;
    if (profile == RSIZ_PROFILE_CINEMA_2K) {
        return ASDCP::MDD_JP2KEssenceCompression_2K;
    }
    if (profile == RSIZ_PROFILE_CINEMA_4K) {
        return ASDCP::MDD_JP2KEssenceCompression_4K;
    }

    const uint16_t family = profile & RSIZ_IMF_FAMILY_MASK;
    const uint8_t main_level = profile & RSIZ_IMF_MAIN_LEVEL_MASK;
    const uint8_t sub_level =
        (profile & RSIZ_IMF_SUB_LEVEL_MASK) >> RSIZ_IMF_SUB_LEVEL_SHIFT;

    const size_t entry_count = sizeof(IMF_ESSENCE_CODINGS) / sizeof(IMF_ESSENCE_CODINGS[0]);
    for (size_t i = 0; i < entry_count; ++i) {
        const imf_essence_coding_entry& entry = IMF_ESSENCE_CODINGS[i];
        if (entry.family == family && entry.main_level == main_level &&
            entry.sub_level == sub_level) {
            return entry.label;
        }
    }

    /* MDD names no label for this level pair, 8K lossy having none at all, so
       signal the profile family without the level. */
    const bool reversible = sub_level == 0;
    switch (family) {
        case RSIZ_IMF_FAMILY_2K:
            return reversible ? ASDCP::MDD_JP2KEssenceCompression_IMFProfile_2K_Reversible
                              : ASDCP::MDD_JP2KEssenceCompression_IMFProfile_2K_Lossy;
        case RSIZ_IMF_FAMILY_4K:
            return reversible ? ASDCP::MDD_JP2KEssenceCompression_IMFProfile_4K_Reversible
                              : ASDCP::MDD_JP2KEssenceCompression_IMFProfile_4K_Lossy;
        case RSIZ_IMF_FAMILY_8K:
            return reversible ? ASDCP::MDD_JP2KEssenceCompression_IMFProfile_8K_Reversible
                              : ASDCP::MDD_JP2KEssenceCompression_IMFProfile_8K_Lossy;
        default:
            /* not an IMF profile at all: as-02-wrap's own default label */
            return ASDCP::MDD_JP2KEssenceCompression_BroadcastProfile_1;
    }
}

void asdcp_jp2k_picture_essence_coding_for_rsize(uint16_t rsize, uint8_t* out_ul) {
    const ASDCP::Dictionary* dict = &ASDCP::DefaultSMPTEDict();
    memcpy(out_ul, dict->ul(essence_coding_for_rsize(rsize)), ASDCP::SMPTE_UL_LENGTH);
}

/* RGBALayout component codes, in codestream component order for an RGB image
   (MXFTypes.h RGBALayoutTable). */
static const byte_t RGB_COMPONENT_CODES[ASDCP_JP2K_MAX_COMPONENTS] = { 'R', 'G', 'B' };

/* SMPTE ST 377-1 puts 0 in the second entry for a progressive image, and a
   full-frame image starts at line 1. This is the pair as-02-wrap writes when
   given "-l 1,0" (as-02-wrap.cpp option -l at line 160, applied to the RGBA
   descriptor at line 1130) and the pair a real IMF App 2 RGBA descriptor
   carries. */
static const ui32_t PROGRESSIVE_FULL_FRAME_LINE_MAP_FIRST = 1;
static const ui32_t PROGRESSIVE_FULL_FRAME_LINE_MAP_SECOND = 0;

/* One RGBALayout entry per codestream component, its code then its precision,
   the remaining entries left as the terminator. */
static bool j2c_layout_from_codestream(const asdcp_codestream_header_t* codestream, byte_t* out) {
    if (codestream->csize == 0 || codestream->csize > ASDCP_JP2K_MAX_COMPONENTS) {
        return false;
    }
    memset(out, 0, ASDCP::MXF::RGBAValueLength);
    for (uint16_t i = 0; i < codestream->csize; ++i) {
        out[i * 2] = RGB_COMPONENT_CODES[i];
        out[i * 2 + 1] = (codestream->image_components[i].ssize & SSIZE_DEPTH_MASK) + 1;
    }
    return true;
}

static const byte_t* rgb_pixel_layout_for_depth(uint8_t depth) {
    switch (depth) {
        case 8:  return ASDCP::MXF::RGBAValue_RGB_8;
        case 10: return ASDCP::MXF::RGBAValue_RGB_10;
        case 12: return ASDCP::MXF::RGBAValue_RGB_12;
        case 16: return ASDCP::MXF::RGBAValue_RGB_16;
        default: return 0;
    }
}

/* Build the AS-02 RGBA descriptor and open for writing. When hdr is non-null its
   HDR/WCG metadata is set on the descriptor before OpenWrite, so it is present in
   the header the writer serializes (SetSourceStream writes it during OpenWrite and
   WriteAS02Footer rewrites it at Finalize). */
static asdcp_result_t as02_jp2k_open_write(asdcp_as02_jp2k_writer_t w, const char* filename,
    const asdcp_writer_info_t* info, const asdcp_picture_descriptor_t* desc,
    const asdcp_hdr_metadata_t* hdr, uint32_t header_size) {
    const ASDCP::Dictionary* dict = &ASDCP::DefaultSMPTEDict();

    ASDCP::WriterInfo wi;
    c_to_cpp_writer_info(info, wi);
    wi.LabelSetType = ASDCP::LS_MXF_SMPTE; // AS-02 is SMPTE-only

    ASDCP::JP2K::PictureDescriptor pd;
    c_to_cpp_picture_desc(desc, pd);

    // Build an RGBA essence descriptor plus a JPEG2000 picture sub-descriptor.
    // The writer takes ownership of both (see AS_02_JP2K.cpp: *i = 0).
    ASDCP::MXF::RGBAEssenceDescriptor* ed = new ASDCP::MXF::RGBAEssenceDescriptor(dict);
    ASDCP::MXF::InterchangeObject_list_t subs;
    subs.push_back(new ASDCP::MXF::JPEG2000PictureSubDescriptor(dict));

    ASDCP::Result_t result = ASDCP::JP2K_PDesc_to_MD(
        pd, *dict,
        *static_cast<ASDCP::MXF::GenericPictureEssenceDescriptor*>(ed),
        *static_cast<ASDCP::MXF::JPEG2000PictureSubDescriptor*>(subs.back()));

    if (ASDCP_FAILURE(result)) {
        delete ed;
        for (ASDCP::MXF::InterchangeObject_list_t::iterator i = subs.begin(); i != subs.end(); ++i) {
            delete *i;
        }
        return result.Value();
    }

    const uint8_t component_depth =
        (desc->codestream.image_components[0].ssize & SSIZE_DEPTH_MASK) + 1;
    const byte_t* pixel_layout = rgb_pixel_layout_for_depth(component_depth);
    byte_t j2c_layout[ASDCP::MXF::RGBAValueLength];
    if (pixel_layout == 0 || !j2c_layout_from_codestream(&desc->codestream, j2c_layout)) {
        delete ed;
        for (ASDCP::MXF::InterchangeObject_list_t::iterator i = subs.begin(); i != subs.end(); ++i) {
            delete *i;
        }
        return ASDCP::RESULT_FORMAT.Value();
    }

    ed->PictureEssenceCoding =
        ASDCP::UL(dict->ul(essence_coding_for_rsize(desc->codestream.rsize)));
    ed->ScanningDirection = 0;
    ed->PixelLayout = ASDCP::MXF::RGBALayout(pixel_layout);
    ed->ComponentMaxRef = (1u << component_depth) - 1;
    ed->ComponentMinRef = 0;
    ed->VideoLineMap = ASDCP::MXF::LineMapPair(PROGRESSIVE_FULL_FRAME_LINE_MAP_FIRST,
        PROGRESSIVE_FULL_FRAME_LINE_MAP_SECOND);
    static_cast<ASDCP::MXF::JPEG2000PictureSubDescriptor*>(subs.back())->J2CLayout =
        ASDCP::MXF::RGBALayout(j2c_layout);

    if (hdr != 0) {
        apply_hdr_metadata(ed, hdr);
    }

    ASDCP::MXF::FileDescriptor* fd = static_cast<ASDCP::MXF::FileDescriptor*>(ed);
    return static_cast<AS_02::JP2K::MXFWriter*>(w)->OpenWrite(
        std::string(filename), wi, fd, subs, pd.EditRate, header_size).Value();
}

asdcp_result_t asdcp_as02_jp2k_writer_open_write(asdcp_as02_jp2k_writer_t w, const char* filename,
    const asdcp_writer_info_t* info, const asdcp_picture_descriptor_t* desc, uint32_t header_size) {
    return as02_jp2k_open_write(w, filename, info, desc, 0, header_size);
}

asdcp_result_t asdcp_as02_jp2k_writer_open_write_hdr(asdcp_as02_jp2k_writer_t w, const char* filename,
    const asdcp_writer_info_t* info, const asdcp_picture_descriptor_t* desc,
    const asdcp_hdr_metadata_t* hdr, uint32_t header_size) {
    return as02_jp2k_open_write(w, filename, info, desc, hdr, header_size);
}

asdcp_result_t asdcp_as02_jp2k_writer_write_frame(asdcp_as02_jp2k_writer_t w,
    const uint8_t* frame_data, uint32_t frame_size,
    asdcp_aes_enc_context_t enc_ctx, asdcp_hmac_context_t hmac_ctx) {
    ASDCP::JP2K::FrameBuffer fb;
    fb.SetData(const_cast<uint8_t*>(frame_data), frame_size);
    fb.Size(frame_size);
    return static_cast<AS_02::JP2K::MXFWriter*>(w)->WriteFrame(
        fb,
        static_cast<ASDCP::AESEncContext*>(enc_ctx),
        static_cast<ASDCP::HMACContext*>(hmac_ctx)
    ).Value();
}

asdcp_result_t asdcp_as02_jp2k_writer_finalize(asdcp_as02_jp2k_writer_t w) {
    return static_cast<AS_02::JP2K::MXFWriter*>(w)->Finalize().Value();
}

/* ---- AS-02 JP2K Reader ---- */
asdcp_as02_jp2k_reader_t asdcp_as02_jp2k_reader_new(void) {
    Kumu::FileReaderFactory defaultFactory;
    return new AS_02::JP2K::MXFReader(defaultFactory);
}

void asdcp_as02_jp2k_reader_free(asdcp_as02_jp2k_reader_t r) {
    delete static_cast<AS_02::JP2K::MXFReader*>(r);
}

asdcp_result_t asdcp_as02_jp2k_reader_open_read(asdcp_as02_jp2k_reader_t r, const char* filename) {
    return static_cast<AS_02::JP2K::MXFReader*>(r)->OpenRead(std::string(filename)).Value();
}

asdcp_result_t asdcp_as02_jp2k_reader_close(asdcp_as02_jp2k_reader_t r) {
    return static_cast<AS_02::JP2K::MXFReader*>(r)->Close().Value();
}

asdcp_result_t asdcp_as02_jp2k_reader_fill_picture_descriptor(asdcp_as02_jp2k_reader_t r, asdcp_picture_descriptor_t* desc) {
    AS_02::JP2K::MXFReader* reader = static_cast<AS_02::JP2K::MXFReader*>(r);
    const ASDCP::Dictionary& dict = ASDCP::DefaultCompositeDict();

    // AS-02 JP2K readers expose no FillPictureDescriptor, so reconstruct the
    // descriptor from header metadata like the upstream as-02-info tool does.
    ASDCP::MXF::InterchangeObject* obj = 0;
    reader->OP1aHeader().GetMDObjectByType(dict.ul(ASDCP::MDD_RGBAEssenceDescriptor), &obj);
    ASDCP::MXF::GenericPictureEssenceDescriptor* ed =
        dynamic_cast<ASDCP::MXF::RGBAEssenceDescriptor*>(obj);
    if (ed == 0) {
        obj = 0;
        reader->OP1aHeader().GetMDObjectByType(dict.ul(ASDCP::MDD_CDCIEssenceDescriptor), &obj);
        ed = dynamic_cast<ASDCP::MXF::CDCIEssenceDescriptor*>(obj);
    }
    if (ed == 0) {
        return ASDCP::RESULT_FORMAT.Value();
    }

    ASDCP::MXF::InterchangeObject* sub_obj = 0;
    reader->OP1aHeader().GetMDObjectByType(dict.ul(ASDCP::MDD_JPEG2000PictureSubDescriptor), &sub_obj);
    ASDCP::MXF::JPEG2000PictureSubDescriptor* sub =
        dynamic_cast<ASDCP::MXF::JPEG2000PictureSubDescriptor*>(sub_obj);
    if (sub == 0) {
        return ASDCP::RESULT_FORMAT.Value();
    }

    std::list<ASDCP::MXF::InterchangeObject*> tracks;
    reader->OP1aHeader().GetMDObjectsByType(dict.ul(ASDCP::MDD_Track), tracks);
    ASDCP::Rational edit_rate;
    if (!tracks.empty()) {
        edit_rate = static_cast<ASDCP::MXF::Track*>(tracks.front())->EditRate;
    }

    ASDCP::JP2K::PictureDescriptor pd;
    ASDCP::Result_t result = ASDCP::MD_to_JP2K_PDesc(*ed, *sub, edit_rate, ed->SampleRate, pd);
    if (ASDCP_SUCCESS(result)) {
        cpp_to_c_picture_desc(pd, desc);
    }
    return result.Value();
}

/* optional_property copy helpers: each writes the has_* flag and leaves the
   value zeroed when the item is absent. */
template <typename PropertyType, typename OutType>
static void copy_optional_number(const ASDCP::MXF::optional_property<PropertyType>& src,
    int32_t* has_value, OutType* out) {
    *has_value = src.empty() ? 0 : 1;
    if (!src.empty()) {
        *out = static_cast<OutType>(src.const_get());
    }
}

static void copy_optional_ul(const ASDCP::MXF::optional_property<ASDCP::UL>& src,
    int32_t* has_value, uint8_t* out) {
    *has_value = src.empty() ? 0 : 1;
    if (!src.empty()) {
        memcpy(out, src.const_get().Value(), ASDCP::SMPTE_UL_LENGTH);
    }
}

/* RGBALayout keeps its bytes private, so archive them out. */
static bool copy_rgba_layout(const ASDCP::MXF::RGBALayout& layout, uint8_t* out) {
    Kumu::MemIOWriter writer(out, ASDCP::MXF::RGBAValueLength);
    return layout.Archive(&writer);
}

static void copy_optional_raw(const ASDCP::MXF::optional_property<ASDCP::MXF::Raw>& src,
    int32_t* has_value, uint32_t* out_length, uint8_t* out) {
    *has_value = src.empty() ? 0 : 1;
    if (src.empty()) {
        return;
    }
    uint32_t length = src.const_get().Length();
    if (length > ASDCP_DESCRIPTOR_RAW_CAPACITY) {
        length = ASDCP_DESCRIPTOR_RAW_CAPACITY;
    }
    memcpy(out, src.const_get().RoData(), length);
    *out_length = length;
}

static void copy_ui16_array(const ASDCP::MXF::optional_property<ASDCP::MXF::Array<Kumu::ArchivableUi16> >& src,
    int32_t* has_value, uint32_t* out_count, uint16_t* out) {
    *has_value = src.empty() ? 0 : 1;
    if (src.empty()) {
        return;
    }
    uint32_t count = 0;
    ASDCP::MXF::Array<Kumu::ArchivableUi16>::const_iterator i;
    for (i = src.const_get().begin(); i != src.const_get().end() && count < ASDCP_JP2K_MAX_PROFILES; ++i) {
        out[count++] = i->value;
    }
    *out_count = count;
}

static ASDCP::MXF::RGBAEssenceDescriptor* as02_rgba_descriptor(asdcp_as02_jp2k_reader_t r) {
    AS_02::JP2K::MXFReader* reader = static_cast<AS_02::JP2K::MXFReader*>(r);
    ASDCP::MXF::InterchangeObject* obj = 0;
    reader->OP1aHeader().GetMDObjectByType(
        ASDCP::DefaultCompositeDict().ul(ASDCP::MDD_RGBAEssenceDescriptor), &obj);
    return dynamic_cast<ASDCP::MXF::RGBAEssenceDescriptor*>(obj);
}

asdcp_result_t asdcp_as02_jp2k_reader_read_rgba_essence_descriptor(asdcp_as02_jp2k_reader_t r,
    asdcp_rgba_essence_descriptor_t* out) {
    ASDCP::MXF::RGBAEssenceDescriptor* ed = as02_rgba_descriptor(r);
    if (ed == 0) {
        return ASDCP::RESULT_FORMAT.Value();
    }

    memset(out, 0, sizeof(*out));
    memcpy(out->instance_id, ed->InstanceUID.Value(), ASDCP::UUIDlen);
    out->has_generation_id = ed->GenerationUID.empty() ? 0 : 1;
    if (!ed->GenerationUID.empty()) {
        memcpy(out->generation_id, ed->GenerationUID.const_get().Value(), ASDCP::UUIDlen);
    }

    for (size_t k = 0; k < ed->Locators.size() && k < ASDCP_MAX_LOCATORS; ++k) {
        memcpy(out->locators[k], ed->Locators[k].Value(), ASDCP::UUIDlen);
        out->locator_count++;
    }
    for (size_t k = 0; k < ed->SubDescriptors.size() && k < ASDCP_MAX_SUB_DESCRIPTORS; ++k) {
        memcpy(out->sub_descriptors[k], ed->SubDescriptors[k].Value(), ASDCP::UUIDlen);
        out->sub_descriptor_count++;
    }

    copy_optional_number(ed->LinkedTrackID, &out->has_linked_track_id, &out->linked_track_id);
    out->sample_rate.numerator = ed->SampleRate.Numerator;
    out->sample_rate.denominator = ed->SampleRate.Denominator;
    copy_optional_number(ed->ContainerDuration, &out->has_container_duration, &out->container_duration);
    memcpy(out->essence_container, ed->EssenceContainer.Value(), ASDCP::SMPTE_UL_LENGTH);
    copy_optional_ul(ed->Codec, &out->has_codec, out->codec);

    copy_optional_number(ed->SignalStandard, &out->has_signal_standard, &out->signal_standard);
    out->frame_layout = ed->FrameLayout;
    out->stored_width = ed->StoredWidth;
    out->stored_height = ed->StoredHeight;
    copy_optional_number(ed->StoredF2Offset, &out->has_stored_f2_offset, &out->stored_f2_offset);
    copy_optional_number(ed->SampledWidth, &out->has_sampled_width, &out->sampled_width);
    copy_optional_number(ed->SampledHeight, &out->has_sampled_height, &out->sampled_height);
    copy_optional_number(ed->SampledXOffset, &out->has_sampled_x_offset, &out->sampled_x_offset);
    copy_optional_number(ed->SampledYOffset, &out->has_sampled_y_offset, &out->sampled_y_offset);
    copy_optional_number(ed->DisplayHeight, &out->has_display_height, &out->display_height);
    copy_optional_number(ed->DisplayWidth, &out->has_display_width, &out->display_width);
    copy_optional_number(ed->DisplayXOffset, &out->has_display_x_offset, &out->display_x_offset);
    copy_optional_number(ed->DisplayYOffset, &out->has_display_y_offset, &out->display_y_offset);
    copy_optional_number(ed->DisplayF2Offset, &out->has_display_f2_offset, &out->display_f2_offset);
    out->aspect_ratio.numerator = ed->AspectRatio.Numerator;
    out->aspect_ratio.denominator = ed->AspectRatio.Denominator;
    copy_optional_number(ed->ActiveFormatDescriptor, &out->has_active_format_descriptor,
        &out->active_format_descriptor);
    copy_optional_number(ed->AlphaTransparency, &out->has_alpha_transparency, &out->alpha_transparency);
    copy_optional_number(ed->ImageAlignmentOffset, &out->has_image_alignment_offset,
        &out->image_alignment_offset);
    copy_optional_number(ed->ImageStartOffset, &out->has_image_start_offset, &out->image_start_offset);
    copy_optional_number(ed->ImageEndOffset, &out->has_image_end_offset, &out->image_end_offset);
    copy_optional_number(ed->FieldDominance, &out->has_field_dominance, &out->field_dominance);
    memcpy(out->picture_essence_coding, ed->PictureEssenceCoding.Value(), ASDCP::SMPTE_UL_LENGTH);
    copy_optional_ul(ed->CodingEquations, &out->has_coding_equations, out->coding_equations);
    if (!ed->AlternativeCenterCuts.empty()) {
        ASDCP::MXF::Batch<ASDCP::UL>::const_iterator i;
        for (i = ed->AlternativeCenterCuts.const_get().begin();
             i != ed->AlternativeCenterCuts.const_get().end() &&
             out->alternative_center_cut_count < ASDCP_MAX_ALTERNATIVE_CENTER_CUTS; ++i) {
            memcpy(out->alternative_center_cuts[out->alternative_center_cut_count],
                i->Value(), ASDCP::SMPTE_UL_LENGTH);
            out->alternative_center_cut_count++;
        }
    }
    copy_optional_number(ed->ActiveWidth, &out->has_active_width, &out->active_width);
    copy_optional_number(ed->ActiveHeight, &out->has_active_height, &out->active_height);
    copy_optional_number(ed->ActiveXOffset, &out->has_active_x_offset, &out->active_x_offset);
    copy_optional_number(ed->ActiveYOffset, &out->has_active_y_offset, &out->active_y_offset);
    out->has_video_line_map = ed->VideoLineMap.empty() ? 0 : 1;
    if (!ed->VideoLineMap.empty()) {
        out->video_line_map[0] = ed->VideoLineMap.const_get().First;
        out->video_line_map[1] = ed->VideoLineMap.const_get().Second;
    }
    read_hdr_metadata(ed, &out->hdr);

    copy_optional_number(ed->ComponentMaxRef, &out->has_component_max_ref, &out->component_max_ref);
    copy_optional_number(ed->ComponentMinRef, &out->has_component_min_ref, &out->component_min_ref);
    copy_optional_number(ed->AlphaMinRef, &out->has_alpha_min_ref, &out->alpha_min_ref);
    copy_optional_number(ed->AlphaMaxRef, &out->has_alpha_max_ref, &out->alpha_max_ref);
    copy_optional_number(ed->ScanningDirection, &out->has_scanning_direction, &out->scanning_direction);
    if (!copy_rgba_layout(ed->PixelLayout, out->pixel_layout)) {
        return ASDCP::RESULT_FORMAT.Value();
    }
    return ASDCP::RESULT_OK.Value();
}

asdcp_result_t asdcp_as02_jp2k_reader_read_rgba_descriptor(asdcp_as02_jp2k_reader_t r,
    asdcp_rgba_descriptor_t* out) {
    asdcp_rgba_essence_descriptor_t full;
    asdcp_result_t result = asdcp_as02_jp2k_reader_read_rgba_essence_descriptor(r, &full);
    if (!ASDCP_SUCCESS(result)) {
        return result;
    }
    memset(out, 0, sizeof(*out));
    out->has_picture_essence_coding = 1;
    memcpy(out->picture_essence_coding, full.picture_essence_coding, ASDCP::SMPTE_UL_LENGTH);
    memcpy(out->pixel_layout, full.pixel_layout, ASDCP::MXF::RGBAValueLength);
    out->has_component_max_ref = full.has_component_max_ref;
    out->component_max_ref = full.component_max_ref;
    out->has_component_min_ref = full.has_component_min_ref;
    out->component_min_ref = full.component_min_ref;
    return result;
}

asdcp_result_t asdcp_as02_jp2k_reader_read_jpeg2000_sub_descriptor(asdcp_as02_jp2k_reader_t r,
    asdcp_jpeg2000_sub_descriptor_t* out) {
    AS_02::JP2K::MXFReader* reader = static_cast<AS_02::JP2K::MXFReader*>(r);
    ASDCP::MXF::InterchangeObject* obj = 0;
    reader->OP1aHeader().GetMDObjectByType(
        ASDCP::DefaultCompositeDict().ul(ASDCP::MDD_JPEG2000PictureSubDescriptor), &obj);
    ASDCP::MXF::JPEG2000PictureSubDescriptor* sub =
        dynamic_cast<ASDCP::MXF::JPEG2000PictureSubDescriptor*>(obj);
    if (sub == 0) {
        return ASDCP::RESULT_FORMAT.Value();
    }

    memset(out, 0, sizeof(*out));
    memcpy(out->instance_id, sub->InstanceUID.Value(), ASDCP::UUIDlen);
    out->has_generation_id = sub->GenerationUID.empty() ? 0 : 1;
    if (!sub->GenerationUID.empty()) {
        memcpy(out->generation_id, sub->GenerationUID.const_get().Value(), ASDCP::UUIDlen);
    }

    out->rsize = sub->Rsize;
    out->xsize = sub->Xsize;
    out->ysize = sub->Ysize;
    out->x_osize = sub->XOsize;
    out->y_osize = sub->YOsize;
    out->xt_size = sub->XTsize;
    out->yt_size = sub->YTsize;
    out->xt_osize = sub->XTOsize;
    out->yt_osize = sub->YTOsize;
    out->csize = sub->Csize;

    copy_optional_raw(sub->PictureComponentSizing, &out->has_picture_component_sizing,
        &out->picture_component_sizing_length, out->picture_component_sizing);
    copy_optional_raw(sub->CodingStyleDefault, &out->has_coding_style_default,
        &out->coding_style_default_length, out->coding_style_default);
    copy_optional_raw(sub->QuantizationDefault, &out->has_quantization_default,
        &out->quantization_default_length, out->quantization_default);

    out->has_j2c_layout = sub->J2CLayout.empty() ? 0 : 1;
    if (!sub->J2CLayout.empty() && !copy_rgba_layout(sub->J2CLayout.const_get(), out->j2c_layout)) {
        return ASDCP::RESULT_FORMAT.Value();
    }

    out->has_extended_capabilities = sub->J2KExtendedCapabilities.empty() ? 0 : 1;
    if (!sub->J2KExtendedCapabilities.empty()) {
        out->pcap = sub->J2KExtendedCapabilities.const_get().Pcap;
        ASDCP::MXF::Array<Kumu::ArchivableUi16>::const_iterator i;
        for (i = sub->J2KExtendedCapabilities.const_get().Ccap.begin();
             i != sub->J2KExtendedCapabilities.const_get().Ccap.end() &&
             out->capability_count < ASDCP_JP2K_MAX_CAPABILITIES; ++i) {
            out->ccap[out->capability_count++] = i->value;
        }
    }
    copy_ui16_array(sub->J2KProfile, &out->has_profile, &out->profile_count, out->profile);
    copy_ui16_array(sub->J2KCorrespondingProfile, &out->has_corresponding_profile,
        &out->corresponding_profile_count, out->corresponding_profile);
    return ASDCP::RESULT_OK.Value();
}

/* Read all HDR/WCG metadata off the AS-02 picture essence descriptor. */
asdcp_result_t asdcp_as02_jp2k_reader_read_hdr(asdcp_as02_jp2k_reader_t r, asdcp_hdr_metadata_t* hdr) {
    AS_02::JP2K::MXFReader* reader = static_cast<AS_02::JP2K::MXFReader*>(r);
    const ASDCP::Dictionary& dict = ASDCP::DefaultCompositeDict();

    memset(hdr, 0, sizeof(*hdr));
    ASDCP::MXF::InterchangeObject* obj = 0;
    reader->OP1aHeader().GetMDObjectByType(dict.ul(ASDCP::MDD_RGBAEssenceDescriptor), &obj);
    if (obj == 0) {
        reader->OP1aHeader().GetMDObjectByType(dict.ul(ASDCP::MDD_CDCIEssenceDescriptor), &obj);
    }
    ASDCP::MXF::GenericPictureEssenceDescriptor* ed =
        dynamic_cast<ASDCP::MXF::GenericPictureEssenceDescriptor*>(obj);
    if (ed == 0) {
        return ASDCP::RESULT_FORMAT.Value();
    }
    read_hdr_metadata(ed, hdr);
    return ASDCP::RESULT_OK.Value();
}

asdcp_result_t asdcp_as02_jp2k_reader_fill_writer_info(asdcp_as02_jp2k_reader_t r, asdcp_writer_info_t* info) {
    ASDCP::WriterInfo wi;
    ASDCP::Result_t result = static_cast<AS_02::JP2K::MXFReader*>(r)->FillWriterInfo(wi);
    if (ASDCP_SUCCESS(result)) {
        cpp_to_c_writer_info(wi, info);
    }
    return result.Value();
}

asdcp_result_t asdcp_as02_jp2k_reader_read_frame(asdcp_as02_jp2k_reader_t r, uint32_t frame_number,
    uint8_t* buf, uint32_t buf_capacity, uint32_t* out_size,
    asdcp_aes_dec_context_t dec_ctx, asdcp_hmac_context_t hmac_ctx) {
    ASDCP::JP2K::FrameBuffer fb;
    fb.SetData(buf, buf_capacity);
    fb.Capacity(buf_capacity);
    ASDCP::Result_t result = static_cast<AS_02::JP2K::MXFReader*>(r)->ReadFrame(
        frame_number, fb,
        static_cast<ASDCP::AESDecContext*>(dec_ctx),
        static_cast<ASDCP::HMACContext*>(hmac_ctx)
    );
    *out_size = fb.Size();
    return result.Value();
}

/* ---- AS-02 PCM Writer ---- */
asdcp_as02_pcm_writer_t asdcp_as02_pcm_writer_new(void) {
    return new AS_02::PCM::MXFWriter();
}

void asdcp_as02_pcm_writer_free(asdcp_as02_pcm_writer_t w) {
    delete static_cast<AS_02::PCM::MXFWriter*>(w);
}

asdcp_result_t asdcp_as02_pcm_writer_open_write(asdcp_as02_pcm_writer_t w, const char* filename,
    const asdcp_writer_info_t* info, const asdcp_audio_descriptor_t* desc, uint32_t header_size) {
    const ASDCP::Dictionary* dict = &ASDCP::DefaultSMPTEDict();

    ASDCP::WriterInfo wi;
    c_to_cpp_writer_info(info, wi);
    wi.LabelSetType = ASDCP::LS_MXF_SMPTE;

    ASDCP::PCM::AudioDescriptor ad;
    c_to_cpp_audio_desc(desc, ad);

    // Writer takes ownership of the WaveAudioDescriptor.
    ASDCP::MXF::WaveAudioDescriptor* ed = new ASDCP::MXF::WaveAudioDescriptor(dict);
    ASDCP::Result_t result = ASDCP::PCM_ADesc_to_MD(ad, ed);
    if (ASDCP_FAILURE(result)) {
        delete ed;
        return result.Value();
    }

    ASDCP::MXF::InterchangeObject_list_t subs; // no MCA labels
    ASDCP::MXF::FileDescriptor* fd = static_cast<ASDCP::MXF::FileDescriptor*>(ed);
    return static_cast<AS_02::PCM::MXFWriter*>(w)->OpenWrite(
        std::string(filename), wi, fd, subs, ad.EditRate, header_size).Value();
}

/* Open an AS-02 PCM MXF with MCA label subdescriptors. Mirrors
   asdcp_pcm_writer_open_write_mca, but the AS-02 OpenWrite already takes the
   subdescriptor list and links it, and the ChannelAssignment is the IMF UL
   rather than the d-cinema one. */
asdcp_result_t asdcp_as02_pcm_writer_open_write_mca(asdcp_as02_pcm_writer_t w, const char* filename,
    const asdcp_writer_info_t* info, const asdcp_audio_descriptor_t* desc,
    const char* mca_config, const char* mca_language, uint32_t header_size) {
    const ASDCP::Dictionary* dict = &ASDCP::DefaultSMPTEDict();

    ASDCP::WriterInfo wi;
    c_to_cpp_writer_info(info, wi);
    wi.LabelSetType = ASDCP::LS_MXF_SMPTE;

    ASDCP::PCM::AudioDescriptor ad;
    c_to_cpp_audio_desc(desc, ad);

    /* AS02_MCAConfigParser is the ASDCP one plus the IMF-only symbols, ST and
       51EX among them, which an IMF audio track needs. */
    ASDCP::MXF::AS02_MCAConfigParser mca(dict);
    const bool decoded = (mca_language == 0 || mca_language[0] == '\0')
        ? mca.DecodeString(std::string(mca_config))
        : mca.DecodeString(std::string(mca_config), std::string(mca_language));
    if (!decoded) {
        return ASDCP::RESULT_FORMAT.Value();
    }

    ASDCP::MXF::InterchangeObject_list_t subs(mca.begin(), mca.end());
    ASDCP::MXF::WaveAudioDescriptor* ed = new ASDCP::MXF::WaveAudioDescriptor(dict);
    ASDCP::Result_t result = ASDCP::PCM_ADesc_to_MD(ad, ed);
    if (ASDCP_SUCCESS(result) && mca.ChannelCount() != ed->ChannelCount) {
        result = ASDCP::RESULT_FORMAT;
    }
    if (ASDCP_FAILURE(result)) {
        delete ed;
        for (ASDCP::MXF::InterchangeObject_list_t::iterator i = subs.begin(); i != subs.end(); ++i) {
            delete *i;
        }
        return result.Value();
    }

    ed->ChannelAssignment = ASDCP::UL(dict->ul(ASDCP::MDD_IMFAudioChannelCfg_MCA));

    ASDCP::MXF::FileDescriptor* fd = static_cast<ASDCP::MXF::FileDescriptor*>(ed);
    result = static_cast<AS_02::PCM::MXFWriter*>(w)->OpenWrite(
        std::string(filename), wi, fd, subs, ad.EditRate, header_size);

    /* OpenWrite zeroes the entries it took ownership of, so anything left is
       still ours to free. */
    for (ASDCP::MXF::InterchangeObject_list_t::iterator i = subs.begin(); i != subs.end(); ++i) {
        delete *i;
    }
    return result.Value();
}

asdcp_result_t asdcp_as02_pcm_writer_write_frame(asdcp_as02_pcm_writer_t w,
    const uint8_t* frame_data, uint32_t frame_size,
    asdcp_aes_enc_context_t enc_ctx, asdcp_hmac_context_t hmac_ctx) {
    ASDCP::PCM::FrameBuffer fb;
    fb.SetData(const_cast<uint8_t*>(frame_data), frame_size);
    fb.Size(frame_size);
    return static_cast<AS_02::PCM::MXFWriter*>(w)->WriteFrame(
        fb,
        static_cast<ASDCP::AESEncContext*>(enc_ctx),
        static_cast<ASDCP::HMACContext*>(hmac_ctx)
    ).Value();
}

asdcp_result_t asdcp_as02_pcm_writer_finalize(asdcp_as02_pcm_writer_t w) {
    return static_cast<AS_02::PCM::MXFWriter*>(w)->Finalize().Value();
}

/* ---- AS-02 PCM Reader ---- */
asdcp_as02_pcm_reader_t asdcp_as02_pcm_reader_new(void) {
    Kumu::FileReaderFactory defaultFactory;
    return new AS_02::PCM::MXFReader(defaultFactory);
}

void asdcp_as02_pcm_reader_free(asdcp_as02_pcm_reader_t r) {
    delete static_cast<AS_02::PCM::MXFReader*>(r);
}

asdcp_result_t asdcp_as02_pcm_reader_open_read(asdcp_as02_pcm_reader_t r, const char* filename,
    int32_t edit_rate_num, int32_t edit_rate_den) {
    return static_cast<AS_02::PCM::MXFReader*>(r)->OpenRead(
        std::string(filename), ASDCP::Rational(edit_rate_num, edit_rate_den)).Value();
}

asdcp_result_t asdcp_as02_pcm_reader_close(asdcp_as02_pcm_reader_t r) {
    return static_cast<AS_02::PCM::MXFReader*>(r)->Close().Value();
}

asdcp_result_t asdcp_as02_pcm_reader_fill_audio_descriptor(asdcp_as02_pcm_reader_t r, asdcp_audio_descriptor_t* desc) {
    AS_02::PCM::MXFReader* reader = static_cast<AS_02::PCM::MXFReader*>(r);
    ASDCP::MXF::InterchangeObject* obj = 0;
    reader->OP1aHeader().GetMDObjectByType(
        ASDCP::DefaultCompositeDict().ul(ASDCP::MDD_WaveAudioDescriptor), &obj);
    ASDCP::MXF::WaveAudioDescriptor* wd = dynamic_cast<ASDCP::MXF::WaveAudioDescriptor*>(obj);
    if (wd == 0) {
        return ASDCP::RESULT_FORMAT.Value();
    }
    ASDCP::PCM::AudioDescriptor ad;
    ASDCP::Result_t result = ASDCP::MD_to_PCM_ADesc(wd, ad);
    if (ASDCP_SUCCESS(result)) {
        cpp_to_c_audio_desc(ad, desc);
    }
    return result.Value();
}

asdcp_result_t asdcp_as02_pcm_reader_fill_writer_info(asdcp_as02_pcm_reader_t r, asdcp_writer_info_t* info) {
    ASDCP::WriterInfo wi;
    ASDCP::Result_t result = static_cast<AS_02::PCM::MXFReader*>(r)->FillWriterInfo(wi);
    if (ASDCP_SUCCESS(result)) {
        cpp_to_c_writer_info(wi, info);
    }
    return result.Value();
}

asdcp_result_t asdcp_as02_pcm_reader_read_frame(asdcp_as02_pcm_reader_t r, uint32_t frame_number,
    uint8_t* buf, uint32_t buf_capacity, uint32_t* out_size,
    asdcp_aes_dec_context_t dec_ctx, asdcp_hmac_context_t hmac_ctx) {
    ASDCP::PCM::FrameBuffer fb;
    fb.SetData(buf, buf_capacity);
    fb.Capacity(buf_capacity);
    ASDCP::Result_t result = static_cast<AS_02::PCM::MXFReader*>(r)->ReadFrame(
        frame_number, fb,
        static_cast<ASDCP::AESDecContext*>(dec_ctx),
        static_cast<ASDCP::HMACContext*>(hmac_ctx)
    );
    *out_size = fb.Size();
    return result.Value();
}

asdcp_result_t asdcp_as02_pcm_reader_read_channel_assignment(asdcp_as02_pcm_reader_t r,
    uint8_t* out_ul, int32_t* present) {
    AS_02::PCM::MXFReader* reader = static_cast<AS_02::PCM::MXFReader*>(r);
    ASDCP::MXF::InterchangeObject* obj = 0;
    reader->OP1aHeader().GetMDObjectByType(
        ASDCP::DefaultCompositeDict().ul(ASDCP::MDD_WaveAudioDescriptor), &obj);
    ASDCP::MXF::WaveAudioDescriptor* wd = dynamic_cast<ASDCP::MXF::WaveAudioDescriptor*>(obj);
    if (wd == 0) {
        return ASDCP::RESULT_FORMAT.Value();
    }
    copy_optional_ul(wd->ChannelAssignment, present, out_ul);
    return ASDCP::RESULT_OK.Value();
}

asdcp_result_t asdcp_as02_pcm_reader_mca_label_count(asdcp_as02_pcm_reader_t r, uint32_t* out_count) {
    std::list<ASDCP::MXF::InterchangeObject*> labels;
    collect_mca_labels(static_cast<AS_02::PCM::MXFReader*>(r)->OP1aHeader(), labels);
    *out_count = static_cast<uint32_t>(labels.size());
    return ASDCP::RESULT_OK.Value();
}

asdcp_result_t asdcp_as02_pcm_reader_mca_label_info(asdcp_as02_pcm_reader_t r, uint32_t index,
    asdcp_mca_label_t* out_label) {
    std::list<ASDCP::MXF::InterchangeObject*> labels;
    collect_mca_labels(static_cast<AS_02::PCM::MXFReader*>(r)->OP1aHeader(), labels);
    return fill_mca_label(labels, index, out_label);
}

/* ---- AS-02 TimedText Writer ---- */
asdcp_as02_timed_text_writer_t asdcp_as02_timed_text_writer_new(void) {
    return new AS_02::TimedText::MXFWriter();
}

void asdcp_as02_timed_text_writer_free(asdcp_as02_timed_text_writer_t w) {
    delete static_cast<AS_02::TimedText::MXFWriter*>(w);
}

asdcp_result_t asdcp_as02_timed_text_writer_open_write(asdcp_as02_timed_text_writer_t w, const char* filename,
    const asdcp_writer_info_t* info, const asdcp_timed_text_descriptor_t* desc, uint32_t header_size) {
    ASDCP::WriterInfo wi;
    c_to_cpp_writer_info(info, wi);
    wi.LabelSetType = ASDCP::LS_MXF_SMPTE;
    ASDCP::TimedText::TimedTextDescriptor td;
    c_to_cpp_timed_text_desc(desc, td);
    return static_cast<AS_02::TimedText::MXFWriter*>(w)->OpenWrite(std::string(filename), wi, td, header_size).Value();
}

asdcp_result_t asdcp_as02_timed_text_writer_write_timed_text_resource(asdcp_as02_timed_text_writer_t w,
    const char* xml_doc,
    asdcp_aes_enc_context_t enc_ctx, asdcp_hmac_context_t hmac_ctx) {
    std::string doc(xml_doc);
    return static_cast<AS_02::TimedText::MXFWriter*>(w)->WriteTimedTextResource(
        doc,
        static_cast<ASDCP::AESEncContext*>(enc_ctx),
        static_cast<ASDCP::HMACContext*>(hmac_ctx)
    ).Value();
}

asdcp_result_t asdcp_as02_timed_text_writer_write_ancillary_resource(asdcp_as02_timed_text_writer_t w,
    const uint8_t* resource_data, uint32_t resource_size,
    const uint8_t* resource_uuid, const char* mime_type,
    asdcp_aes_enc_context_t enc_ctx, asdcp_hmac_context_t hmac_ctx) {
    ASDCP::TimedText::FrameBuffer fb;
    fb.SetData(const_cast<uint8_t*>(resource_data), resource_size);
    fb.Size(resource_size);
    fb.AssetID(resource_uuid);
    fb.MIMEType(std::string(mime_type));
    return static_cast<AS_02::TimedText::MXFWriter*>(w)->WriteAncillaryResource(
        fb,
        static_cast<ASDCP::AESEncContext*>(enc_ctx),
        static_cast<ASDCP::HMACContext*>(hmac_ctx)
    ).Value();
}

asdcp_result_t asdcp_as02_timed_text_writer_finalize(asdcp_as02_timed_text_writer_t w) {
    return static_cast<AS_02::TimedText::MXFWriter*>(w)->Finalize().Value();
}

/* ---- AS-02 TimedText Reader ---- */
asdcp_as02_timed_text_reader_t asdcp_as02_timed_text_reader_new(void) {
    Kumu::FileReaderFactory defaultFactory;
    return new AS_02::TimedText::MXFReader(defaultFactory);
}

void asdcp_as02_timed_text_reader_free(asdcp_as02_timed_text_reader_t r) {
    delete static_cast<AS_02::TimedText::MXFReader*>(r);
}

asdcp_result_t asdcp_as02_timed_text_reader_open_read(asdcp_as02_timed_text_reader_t r, const char* filename) {
    return static_cast<AS_02::TimedText::MXFReader*>(r)->OpenRead(std::string(filename)).Value();
}

asdcp_result_t asdcp_as02_timed_text_reader_close(asdcp_as02_timed_text_reader_t r) {
    return static_cast<AS_02::TimedText::MXFReader*>(r)->Close().Value();
}

asdcp_result_t asdcp_as02_timed_text_reader_fill_descriptor(asdcp_as02_timed_text_reader_t r, asdcp_timed_text_descriptor_t* desc) {
    ASDCP::TimedText::TimedTextDescriptor td;
    ASDCP::Result_t result = static_cast<AS_02::TimedText::MXFReader*>(r)->FillTimedTextDescriptor(td);
    if (ASDCP_SUCCESS(result)) {
        cpp_to_c_timed_text_desc(td, desc);
    }
    return result.Value();
}

asdcp_result_t asdcp_as02_timed_text_reader_fill_writer_info(asdcp_as02_timed_text_reader_t r, asdcp_writer_info_t* info) {
    ASDCP::WriterInfo wi;
    ASDCP::Result_t result = static_cast<AS_02::TimedText::MXFReader*>(r)->FillWriterInfo(wi);
    if (ASDCP_SUCCESS(result)) {
        cpp_to_c_writer_info(wi, info);
    }
    return result.Value();
}

asdcp_result_t asdcp_as02_timed_text_reader_read_timed_text_resource(asdcp_as02_timed_text_reader_t r,
    uint8_t* buf, uint32_t buf_capacity, uint32_t* out_size,
    asdcp_aes_dec_context_t dec_ctx, asdcp_hmac_context_t hmac_ctx) {
    std::string doc;
    ASDCP::Result_t result = static_cast<AS_02::TimedText::MXFReader*>(r)->ReadTimedTextResource(
        doc,
        static_cast<ASDCP::AESDecContext*>(dec_ctx),
        static_cast<ASDCP::HMACContext*>(hmac_ctx)
    );
    if (ASDCP_SUCCESS(result)) {
        uint32_t doc_len = static_cast<uint32_t>(doc.size());
        *out_size = doc_len;
        if (doc_len > buf_capacity) {
            return ASDCP::RESULT_SMALLBUF.Value();
        }
        memcpy(buf, doc.data(), doc_len);
    } else {
        *out_size = 0;
    }
    return result.Value();
}

/* ---- Utility ---- */
int32_t asdcp_result_ok(asdcp_result_t result) {
    return ASDCP_SUCCESS(result) ? 1 : 0;
}

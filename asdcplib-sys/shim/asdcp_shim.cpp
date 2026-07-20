/* asdcplib C shim implementation */
#include "asdcp_shim.h"
#include <AS_DCP.h>
#include <AS_02.h>
#include <Metadata.h>
#include <MDD.h>
#include <KM_fileio.h>
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

/* Helper: convert C picture desc to C++ */
static void c_to_cpp_picture_desc(const asdcp_picture_descriptor_t* c, ASDCP::JP2K::PictureDescriptor& cpp) {
    cpp = ASDCP::JP2K::PictureDescriptor();
    cpp.EditRate = ASDCP::Rational(c->edit_rate.numerator, c->edit_rate.denominator);
    cpp.SampleRate = ASDCP::Rational(c->sample_rate.numerator, c->sample_rate.denominator);
    cpp.StoredWidth = c->stored_width;
    cpp.StoredHeight = c->stored_height;
    cpp.AspectRatio = ASDCP::Rational(c->aspect_ratio.numerator, c->aspect_ratio.denominator);
    cpp.ContainerDuration = c->container_duration;
    cpp.Csize = c->csize;
}

static void cpp_to_c_picture_desc(const ASDCP::JP2K::PictureDescriptor& cpp, asdcp_picture_descriptor_t* c) {
    c->edit_rate.numerator = cpp.EditRate.Numerator;
    c->edit_rate.denominator = cpp.EditRate.Denominator;
    c->sample_rate.numerator = cpp.SampleRate.Numerator;
    c->sample_rate.denominator = cpp.SampleRate.Denominator;
    c->stored_width = cpp.StoredWidth;
    c->stored_height = cpp.StoredHeight;
    c->aspect_ratio.numerator = cpp.AspectRatio.Numerator;
    c->aspect_ratio.denominator = cpp.AspectRatio.Denominator;
    c->container_duration = cpp.ContainerDuration;
    c->csize = cpp.Csize;
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
    const char* xml_doc, uint32_t xml_len,
    asdcp_aes_enc_context_t enc_ctx, asdcp_hmac_context_t hmac_ctx) {
    (void)xml_len;
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

asdcp_result_t asdcp_as02_jp2k_writer_open_write(asdcp_as02_jp2k_writer_t w, const char* filename,
    const asdcp_writer_info_t* info, const asdcp_picture_descriptor_t* desc, uint32_t header_size) {
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

    ed->PictureEssenceCoding = ASDCP::UL(dict->ul(ASDCP::MDD_JP2KEssenceCompression_BroadcastProfile_1));
    ed->ScanningDirection = 0;
    ed->PixelLayout = ASDCP::MXF::RGBALayout(ASDCP::MXF::RGBAValue_RGB_8);

    ASDCP::MXF::FileDescriptor* fd = static_cast<ASDCP::MXF::FileDescriptor*>(ed);
    return static_cast<AS_02::JP2K::MXFWriter*>(w)->OpenWrite(
        std::string(filename), wi, fd, subs, pd.EditRate, header_size).Value();
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
    const char* xml_doc, uint32_t xml_len,
    asdcp_aes_enc_context_t enc_ctx, asdcp_hmac_context_t hmac_ctx) {
    (void)xml_len;
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

/* asdcplib C shim — exposes C++ asdcplib classes through a C-compatible interface */
#ifndef ASDCP_SHIM_H
#define ASDCP_SHIM_H

#include <stdint.h>

#ifdef __cplusplus
extern "C" {
#endif

typedef int32_t asdcp_result_t;

typedef struct {
    uint8_t product_uuid[16];
    uint8_t asset_uuid[16];
    uint8_t context_id[16];
    uint8_t cryptographic_key_id[16];
    int32_t encrypted_essence;
    int32_t uses_hmac;
    int32_t label_set_type; /* 0=Unknown, 1=Interop, 2=SMPTE */
} asdcp_writer_info_t;

typedef struct {
    int32_t numerator;
    int32_t denominator;
} asdcp_rational_t;

typedef struct {
    asdcp_rational_t edit_rate;
    asdcp_rational_t sample_rate;
    uint32_t stored_width;
    uint32_t stored_height;
    asdcp_rational_t aspect_ratio;
    uint32_t container_duration;
    uint16_t csize;
} asdcp_picture_descriptor_t;

typedef struct {
    asdcp_rational_t edit_rate;
    asdcp_rational_t audio_sampling_rate;
    uint32_t locked;
    uint32_t channel_count;
    uint32_t quantization_bits;
    uint32_t block_align;
    uint32_t avg_bps;
    uint32_t linked_track_id;
    uint32_t container_duration;
    int32_t channel_format;
} asdcp_audio_descriptor_t;

typedef struct {
    asdcp_rational_t edit_rate;
    uint32_t container_duration;
    uint8_t asset_id[16];
} asdcp_timed_text_descriptor_t;

typedef struct {
    asdcp_rational_t edit_rate;
    uint32_t container_duration;
    uint8_t asset_id[16];
    uint8_t data_essence_coding[16];
    uint32_t first_frame;
    uint16_t max_channel_count;
    uint16_t max_object_count;
    uint8_t atmos_id[16];
    uint8_t atmos_version;
} asdcp_atmos_descriptor_t;

typedef void* asdcp_jp2k_writer_t;
typedef void* asdcp_jp2k_reader_t;
typedef void* asdcp_pcm_writer_t;
typedef void* asdcp_pcm_reader_t;
typedef void* asdcp_timed_text_writer_t;
typedef void* asdcp_timed_text_reader_t;
typedef void* asdcp_atmos_writer_t;
typedef void* asdcp_atmos_reader_t;
typedef void* asdcp_jp2k_s_writer_t;
typedef void* asdcp_jp2k_s_reader_t;
typedef void* asdcp_aes_enc_context_t;
typedef void* asdcp_aes_dec_context_t;
typedef void* asdcp_hmac_context_t;

/* Version */
const char* asdcp_version(void);

/* Essence type detection */
asdcp_result_t asdcp_essence_type(const char* filename, int32_t* out_type);
asdcp_result_t asdcp_raw_essence_type(const char* filename, int32_t* out_type);

/* AES Encryption Context */
asdcp_aes_enc_context_t asdcp_aes_enc_context_new(void);
void asdcp_aes_enc_context_free(asdcp_aes_enc_context_t ctx);
asdcp_result_t asdcp_aes_enc_context_init_key(asdcp_aes_enc_context_t ctx, const uint8_t* key);
asdcp_result_t asdcp_aes_enc_context_set_ivec(asdcp_aes_enc_context_t ctx, const uint8_t* ivec);

/* AES Decryption Context */
asdcp_aes_dec_context_t asdcp_aes_dec_context_new(void);
void asdcp_aes_dec_context_free(asdcp_aes_dec_context_t ctx);
asdcp_result_t asdcp_aes_dec_context_init_key(asdcp_aes_dec_context_t ctx, const uint8_t* key);
asdcp_result_t asdcp_aes_dec_context_set_ivec(asdcp_aes_dec_context_t ctx, const uint8_t* ivec);

/* HMAC Context */
asdcp_hmac_context_t asdcp_hmac_context_new(void);
void asdcp_hmac_context_free(asdcp_hmac_context_t ctx);
asdcp_result_t asdcp_hmac_context_init_key(asdcp_hmac_context_t ctx, const uint8_t* key, int32_t label_set);

/* JP2K Writer */
asdcp_jp2k_writer_t asdcp_jp2k_writer_new(void);
void asdcp_jp2k_writer_free(asdcp_jp2k_writer_t w);
asdcp_result_t asdcp_jp2k_writer_open_write(asdcp_jp2k_writer_t w, const char* filename,
    const asdcp_writer_info_t* info, const asdcp_picture_descriptor_t* desc, uint32_t header_size);
asdcp_result_t asdcp_jp2k_writer_write_frame(asdcp_jp2k_writer_t w,
    const uint8_t* frame_data, uint32_t frame_size,
    asdcp_aes_enc_context_t enc_ctx, asdcp_hmac_context_t hmac_ctx);
asdcp_result_t asdcp_jp2k_writer_finalize(asdcp_jp2k_writer_t w);

/* JP2K Reader */
asdcp_jp2k_reader_t asdcp_jp2k_reader_new(void);
void asdcp_jp2k_reader_free(asdcp_jp2k_reader_t r);
asdcp_result_t asdcp_jp2k_reader_open_read(asdcp_jp2k_reader_t r, const char* filename);
asdcp_result_t asdcp_jp2k_reader_close(asdcp_jp2k_reader_t r);
asdcp_result_t asdcp_jp2k_reader_fill_picture_descriptor(asdcp_jp2k_reader_t r, asdcp_picture_descriptor_t* desc);
asdcp_result_t asdcp_jp2k_reader_fill_writer_info(asdcp_jp2k_reader_t r, asdcp_writer_info_t* info);
asdcp_result_t asdcp_jp2k_reader_read_frame(asdcp_jp2k_reader_t r, uint32_t frame_number,
    uint8_t* buf, uint32_t buf_capacity, uint32_t* out_size,
    asdcp_aes_dec_context_t dec_ctx, asdcp_hmac_context_t hmac_ctx);

/* PCM Writer */
asdcp_pcm_writer_t asdcp_pcm_writer_new(void);
void asdcp_pcm_writer_free(asdcp_pcm_writer_t w);
asdcp_result_t asdcp_pcm_writer_open_write(asdcp_pcm_writer_t w, const char* filename,
    const asdcp_writer_info_t* info, const asdcp_audio_descriptor_t* desc, uint32_t header_size);
asdcp_result_t asdcp_pcm_writer_write_frame(asdcp_pcm_writer_t w,
    const uint8_t* frame_data, uint32_t frame_size,
    asdcp_aes_enc_context_t enc_ctx, asdcp_hmac_context_t hmac_ctx);
asdcp_result_t asdcp_pcm_writer_finalize(asdcp_pcm_writer_t w);

/* PCM Reader */
asdcp_pcm_reader_t asdcp_pcm_reader_new(void);
void asdcp_pcm_reader_free(asdcp_pcm_reader_t r);
asdcp_result_t asdcp_pcm_reader_open_read(asdcp_pcm_reader_t r, const char* filename);
asdcp_result_t asdcp_pcm_reader_close(asdcp_pcm_reader_t r);
asdcp_result_t asdcp_pcm_reader_fill_audio_descriptor(asdcp_pcm_reader_t r, asdcp_audio_descriptor_t* desc);
asdcp_result_t asdcp_pcm_reader_fill_writer_info(asdcp_pcm_reader_t r, asdcp_writer_info_t* info);
asdcp_result_t asdcp_pcm_reader_read_frame(asdcp_pcm_reader_t r, uint32_t frame_number,
    uint8_t* buf, uint32_t buf_capacity, uint32_t* out_size,
    asdcp_aes_dec_context_t dec_ctx, asdcp_hmac_context_t hmac_ctx);

/* TimedText Writer */
asdcp_timed_text_writer_t asdcp_timed_text_writer_new(void);
void asdcp_timed_text_writer_free(asdcp_timed_text_writer_t w);
asdcp_result_t asdcp_timed_text_writer_open_write(asdcp_timed_text_writer_t w, const char* filename,
    const asdcp_writer_info_t* info, const asdcp_timed_text_descriptor_t* desc, uint32_t header_size);
asdcp_result_t asdcp_timed_text_writer_write_timed_text_resource(asdcp_timed_text_writer_t w,
    const char* xml_doc, uint32_t xml_len,
    asdcp_aes_enc_context_t enc_ctx, asdcp_hmac_context_t hmac_ctx);
asdcp_result_t asdcp_timed_text_writer_write_ancillary_resource(asdcp_timed_text_writer_t w,
    const uint8_t* resource_data, uint32_t resource_size,
    const uint8_t* resource_uuid, const char* mime_type,
    asdcp_aes_enc_context_t enc_ctx, asdcp_hmac_context_t hmac_ctx);
asdcp_result_t asdcp_timed_text_writer_finalize(asdcp_timed_text_writer_t w);

/* TimedText Reader */
asdcp_timed_text_reader_t asdcp_timed_text_reader_new(void);
void asdcp_timed_text_reader_free(asdcp_timed_text_reader_t r);
asdcp_result_t asdcp_timed_text_reader_open_read(asdcp_timed_text_reader_t r, const char* filename);
asdcp_result_t asdcp_timed_text_reader_close(asdcp_timed_text_reader_t r);
asdcp_result_t asdcp_timed_text_reader_fill_descriptor(asdcp_timed_text_reader_t r, asdcp_timed_text_descriptor_t* desc);
asdcp_result_t asdcp_timed_text_reader_fill_writer_info(asdcp_timed_text_reader_t r, asdcp_writer_info_t* info);
asdcp_result_t asdcp_timed_text_reader_read_timed_text_resource(asdcp_timed_text_reader_t r,
    uint8_t* buf, uint32_t buf_capacity, uint32_t* out_size,
    asdcp_aes_dec_context_t dec_ctx, asdcp_hmac_context_t hmac_ctx);

/* Atmos Writer */
asdcp_atmos_writer_t asdcp_atmos_writer_new(void);
void asdcp_atmos_writer_free(asdcp_atmos_writer_t w);
asdcp_result_t asdcp_atmos_writer_open_write(asdcp_atmos_writer_t w, const char* filename,
    const asdcp_writer_info_t* info, const asdcp_atmos_descriptor_t* desc, uint32_t header_size);
asdcp_result_t asdcp_atmos_writer_write_frame(asdcp_atmos_writer_t w,
    const uint8_t* frame_data, uint32_t frame_size,
    asdcp_aes_enc_context_t enc_ctx, asdcp_hmac_context_t hmac_ctx);
asdcp_result_t asdcp_atmos_writer_finalize(asdcp_atmos_writer_t w);

/* Atmos Reader */
asdcp_atmos_reader_t asdcp_atmos_reader_new(void);
void asdcp_atmos_reader_free(asdcp_atmos_reader_t r);
asdcp_result_t asdcp_atmos_reader_open_read(asdcp_atmos_reader_t r, const char* filename);
asdcp_result_t asdcp_atmos_reader_close(asdcp_atmos_reader_t r);
asdcp_result_t asdcp_atmos_reader_fill_atmos_descriptor(asdcp_atmos_reader_t r, asdcp_atmos_descriptor_t* desc);
asdcp_result_t asdcp_atmos_reader_fill_writer_info(asdcp_atmos_reader_t r, asdcp_writer_info_t* info);
asdcp_result_t asdcp_atmos_reader_read_frame(asdcp_atmos_reader_t r, uint32_t frame_number,
    uint8_t* buf, uint32_t buf_capacity, uint32_t* out_size,
    asdcp_aes_dec_context_t dec_ctx, asdcp_hmac_context_t hmac_ctx);

/* JP2K Stereoscopic Writer */
asdcp_jp2k_s_writer_t asdcp_jp2k_s_writer_new(void);
void asdcp_jp2k_s_writer_free(asdcp_jp2k_s_writer_t w);
asdcp_result_t asdcp_jp2k_s_writer_open_write(asdcp_jp2k_s_writer_t w, const char* filename,
    const asdcp_writer_info_t* info, const asdcp_picture_descriptor_t* desc, uint32_t header_size);
asdcp_result_t asdcp_jp2k_s_writer_write_frame(asdcp_jp2k_s_writer_t w,
    const uint8_t* frame_data, uint32_t frame_size, int32_t phase,
    asdcp_aes_enc_context_t enc_ctx, asdcp_hmac_context_t hmac_ctx);
asdcp_result_t asdcp_jp2k_s_writer_finalize(asdcp_jp2k_s_writer_t w);

/* JP2K Stereoscopic Reader */
asdcp_jp2k_s_reader_t asdcp_jp2k_s_reader_new(void);
void asdcp_jp2k_s_reader_free(asdcp_jp2k_s_reader_t r);
asdcp_result_t asdcp_jp2k_s_reader_open_read(asdcp_jp2k_s_reader_t r, const char* filename);
asdcp_result_t asdcp_jp2k_s_reader_close(asdcp_jp2k_s_reader_t r);
asdcp_result_t asdcp_jp2k_s_reader_fill_picture_descriptor(asdcp_jp2k_s_reader_t r, asdcp_picture_descriptor_t* desc);
asdcp_result_t asdcp_jp2k_s_reader_fill_writer_info(asdcp_jp2k_s_reader_t r, asdcp_writer_info_t* info);
asdcp_result_t asdcp_jp2k_s_reader_read_frame(asdcp_jp2k_s_reader_t r, uint32_t frame_number,
    int32_t phase, uint8_t* buf, uint32_t buf_capacity, uint32_t* out_size,
    asdcp_aes_dec_context_t dec_ctx, asdcp_hmac_context_t hmac_ctx);

/* Utility */
int32_t asdcp_result_ok(asdcp_result_t result);

#ifdef __cplusplus
}
#endif

#endif /* ASDCP_SHIM_H */

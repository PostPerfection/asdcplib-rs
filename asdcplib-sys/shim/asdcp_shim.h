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

/* Mirrors of ASDCP::JP2K::MaxComponents, MaxPrecincts, MaxDefaults and
   MaxCapabilities. */
#define ASDCP_JP2K_MAX_COMPONENTS 3
#define ASDCP_JP2K_MAX_PRECINCT_SIZES 32
#define ASDCP_JP2K_MAX_QUANTIZATION_STEPS 256
#define ASDCP_JP2K_MAX_CAPABILITIES 32

/* ASDCP::JP2K::NoExtendedCapabilitiesSignaled: capability_count when the
   codestream has no CAP marker, which keeps J2KExtendedCapabilities off the
   sub-descriptor entirely. */
#define ASDCP_JP2K_NO_EXTENDED_CAPABILITIES (-1)

/* One SIZ component (ISO 15444-1 Annex A.5.1). ssize holds the bit depth minus
   one in its low 7 bits and the signed flag in the top bit. */
typedef struct {
    uint8_t ssize;
    uint8_t x_rsize;
    uint8_t y_rsize;
} asdcp_image_component_t;

/* The COD marker segment (ISO 15444-1 Annex A.6.1), flattened from asdcplib's
   nested SGcod/SPcod structs. number_of_layers is the raw big-endian pair. */
typedef struct {
    uint8_t scod;
    uint8_t progression_order;
    uint8_t number_of_layers[2];
    uint8_t multi_component_transform;
    uint8_t decomposition_levels;
    uint8_t codeblock_width;
    uint8_t codeblock_height;
    uint8_t codeblock_style;
    uint8_t transformation;
    uint8_t precinct_sizes[ASDCP_JP2K_MAX_PRECINCT_SIZES];
} asdcp_coding_style_default_t;

/* The QCD marker segment (ISO 15444-1 Annex A.6.4). */
typedef struct {
    uint8_t sqcd;
    uint8_t spqcd[ASDCP_JP2K_MAX_QUANTIZATION_STEPS];
    uint8_t spqcd_length;
} asdcp_quantization_default_t;

/* The CAP marker segment (ISO 15444-1 Annex A.5.2). */
typedef struct {
    uint32_t pcap;
    int8_t capability_count;
    uint16_t ccap[ASDCP_JP2K_MAX_CAPABILITIES];
} asdcp_extended_capabilities_t;

/* The JPEG 2000 codestream header values that go into the MXF
   JPEG2000PictureSubDescriptor. Filled by asdcp_jp2k_parse_codestream_header
   from a real codestream, never by hand. */
typedef struct {
    uint16_t rsize;
    uint32_t xsize;
    uint32_t ysize;
    uint32_t x_osize;
    uint32_t y_osize;
    uint32_t xt_size;
    uint32_t yt_size;
    uint32_t xt_osize;
    uint32_t yt_osize;
    uint16_t csize;
    asdcp_image_component_t image_components[ASDCP_JP2K_MAX_COMPONENTS];
    asdcp_coding_style_default_t coding_style_default;
    asdcp_quantization_default_t quantization_default;
    asdcp_extended_capabilities_t extended_capabilities;
} asdcp_codestream_header_t;

typedef struct {
    asdcp_rational_t edit_rate;
    asdcp_rational_t sample_rate;
    uint32_t stored_width;
    uint32_t stored_height;
    asdcp_rational_t aspect_ratio;
    uint32_t container_duration;
    asdcp_codestream_header_t codestream;
} asdcp_picture_descriptor_t;

/* Parse a JPEG 2000 codestream's SIZ, COD and QCD markers into out. Returns
   RESULT_RAW_ESS or RESULT_RAW_FORMAT if the bytes are not a codestream. */
asdcp_result_t asdcp_jp2k_parse_codestream_header(const uint8_t* codestream, uint32_t size,
    asdcp_codestream_header_t* out);

/* Write into out_ul the 16-byte PictureEssenceCoding label the AS-02 writer
   would set for this Rsiz. */
void asdcp_jp2k_picture_essence_coding_for_rsize(uint16_t rsize, uint8_t* out_ul);

/* The RGBA essence descriptor properties an AS-02 picture track carries beyond
   the shared picture descriptor. pixel_layout is the SMPTE 377 component code
   and depth pairs. */
typedef struct {
    int32_t has_picture_essence_coding;
    uint8_t picture_essence_coding[16];
    uint8_t pixel_layout[16];
    int32_t has_component_max_ref;
    uint32_t component_max_ref;
    int32_t has_component_min_ref;
    uint32_t component_min_ref;
} asdcp_rgba_descriptor_t;

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

/* Holds a timed-text NamespaceURI or UCSEncoding, NUL-terminated and truncated
   to fit; an IMSC profile designator is well under this. */
#define ASDCP_TIMED_TEXT_STRING_CAPACITY 256

typedef struct {
    asdcp_rational_t edit_rate;
    uint32_t container_duration;
    uint8_t asset_id[16];
    char namespace_uri[ASDCP_TIMED_TEXT_STRING_CAPACITY];
    char ucs_encoding[ASDCP_TIMED_TEXT_STRING_CAPACITY];
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

/* HDR/WCG picture metadata (ST 2067-21). Each field is written or read only when
   its has_* flag is set. Chromaticity coordinates are the raw ST 2086 ui16 values
   (0.00002 increments); luminance is the raw ui32 (0.0001 cd/m^2 increments).
   mastering_display_primaries is First.x,First.y,Second.x,Second.y,Third.x,Third.y. */
typedef struct {
    int32_t has_transfer_characteristic;
    uint8_t transfer_characteristic[16];
    int32_t has_color_primaries;
    uint8_t color_primaries[16];
    int32_t has_mastering_display_primaries;
    uint16_t mastering_display_primaries[6];
    int32_t has_mastering_display_white_point;
    uint16_t mastering_display_white_point[2];
    int32_t has_mastering_display_max_luminance;
    uint32_t mastering_display_max_luminance;
    int32_t has_mastering_display_min_luminance;
    uint32_t mastering_display_min_luminance;
} asdcp_hdr_metadata_t;

/* Room for the strong references a picture essence descriptor holds. AS-02 JP2K
   writes one sub-descriptor and no locators; the extra room is for files this
   library did not write. */
#define ASDCP_MAX_SUB_DESCRIPTORS 16
#define ASDCP_MAX_LOCATORS 8
#define ASDCP_MAX_ALTERNATIVE_CENTER_CUTS 8

/* Every item of ASDCP::MXF::RGBAEssenceDescriptor and the classes it derives
   from, so a caller can repeat the whole descriptor in an IMF CPL
   EssenceDescriptorList. Each has_* flag guards the field below it; the items
   without a flag are mandatory in asdcplib and always written. hdr carries
   TransferCharacteristic, ColorPrimaries and the ST 2086 mastering display
   block, which live on GenericPictureEssenceDescriptor. */
typedef struct {
    uint8_t instance_id[16];
    int32_t has_generation_id;
    uint8_t generation_id[16];

    uint32_t locator_count;
    uint8_t locators[ASDCP_MAX_LOCATORS][16];
    uint32_t sub_descriptor_count;
    uint8_t sub_descriptors[ASDCP_MAX_SUB_DESCRIPTORS][16];

    int32_t has_linked_track_id;
    uint32_t linked_track_id;
    asdcp_rational_t sample_rate;
    int32_t has_container_duration;
    uint64_t container_duration;
    uint8_t essence_container[16];
    int32_t has_codec;
    uint8_t codec[16];

    int32_t has_signal_standard;
    uint8_t signal_standard;
    uint8_t frame_layout;
    uint32_t stored_width;
    uint32_t stored_height;
    int32_t has_stored_f2_offset;
    uint32_t stored_f2_offset;
    int32_t has_sampled_width;
    uint32_t sampled_width;
    int32_t has_sampled_height;
    uint32_t sampled_height;
    int32_t has_sampled_x_offset;
    uint32_t sampled_x_offset;
    int32_t has_sampled_y_offset;
    uint32_t sampled_y_offset;
    int32_t has_display_height;
    uint32_t display_height;
    int32_t has_display_width;
    uint32_t display_width;
    int32_t has_display_x_offset;
    uint32_t display_x_offset;
    int32_t has_display_y_offset;
    uint32_t display_y_offset;
    int32_t has_display_f2_offset;
    uint32_t display_f2_offset;
    asdcp_rational_t aspect_ratio;
    int32_t has_active_format_descriptor;
    uint8_t active_format_descriptor;
    int32_t has_alpha_transparency;
    uint8_t alpha_transparency;
    int32_t has_image_alignment_offset;
    uint32_t image_alignment_offset;
    int32_t has_image_start_offset;
    uint32_t image_start_offset;
    int32_t has_image_end_offset;
    uint32_t image_end_offset;
    int32_t has_field_dominance;
    uint8_t field_dominance;
    uint8_t picture_essence_coding[16];
    int32_t has_coding_equations;
    uint8_t coding_equations[16];
    uint32_t alternative_center_cut_count;
    uint8_t alternative_center_cuts[ASDCP_MAX_ALTERNATIVE_CENTER_CUTS][16];
    int32_t has_active_width;
    uint32_t active_width;
    int32_t has_active_height;
    uint32_t active_height;
    int32_t has_active_x_offset;
    uint32_t active_x_offset;
    int32_t has_active_y_offset;
    uint32_t active_y_offset;
    int32_t has_video_line_map;
    uint32_t video_line_map[2];
    asdcp_hdr_metadata_t hdr;

    int32_t has_component_max_ref;
    uint32_t component_max_ref;
    int32_t has_component_min_ref;
    uint32_t component_min_ref;
    int32_t has_alpha_min_ref;
    uint32_t alpha_min_ref;
    int32_t has_alpha_max_ref;
    uint32_t alpha_max_ref;
    int32_t has_scanning_direction;
    uint8_t scanning_direction;
    uint8_t pixel_layout[16];
} asdcp_rgba_essence_descriptor_t;

/* Holds the longest raw marker segment the sub-descriptor carries,
   QuantizationDefault at Sqcd plus MaxDefaults step sizes. */
#define ASDCP_DESCRIPTOR_RAW_CAPACITY (ASDCP_JP2K_MAX_QUANTIZATION_STEPS + 1)

/* Mirrors of ASDCP::JP2K::MaxPRFN and MaxCPFN. */
#define ASDCP_JP2K_MAX_PROFILES 4

/* Every item of ASDCP::MXF::JPEG2000PictureSubDescriptor. The three raw marker
   segments and the two profile arrays carry their own length, and every has_*
   flag guards the field below it. */
typedef struct {
    uint8_t instance_id[16];
    int32_t has_generation_id;
    uint8_t generation_id[16];

    uint16_t rsize;
    uint32_t xsize;
    uint32_t ysize;
    uint32_t x_osize;
    uint32_t y_osize;
    uint32_t xt_size;
    uint32_t yt_size;
    uint32_t xt_osize;
    uint32_t yt_osize;
    uint16_t csize;

    int32_t has_picture_component_sizing;
    uint32_t picture_component_sizing_length;
    uint8_t picture_component_sizing[ASDCP_DESCRIPTOR_RAW_CAPACITY];
    int32_t has_coding_style_default;
    uint32_t coding_style_default_length;
    uint8_t coding_style_default[ASDCP_DESCRIPTOR_RAW_CAPACITY];
    int32_t has_quantization_default;
    uint32_t quantization_default_length;
    uint8_t quantization_default[ASDCP_DESCRIPTOR_RAW_CAPACITY];

    int32_t has_j2c_layout;
    uint8_t j2c_layout[16];

    int32_t has_extended_capabilities;
    uint32_t pcap;
    uint32_t capability_count;
    uint16_t ccap[ASDCP_JP2K_MAX_CAPABILITIES];
    int32_t has_profile;
    uint32_t profile_count;
    uint16_t profile[ASDCP_JP2K_MAX_PROFILES];
    int32_t has_corresponding_profile;
    uint32_t corresponding_profile_count;
    uint16_t corresponding_profile[ASDCP_JP2K_MAX_PROFILES];
} asdcp_jpeg2000_sub_descriptor_t;

/* Room for the MCA label subdescriptors a sound track file links: one
   soundfield group plus one per channel, and ST 2067-2 allows 64 channels. */
#define ASDCP_MAX_SOUND_SUB_DESCRIPTORS 72

/* Every item of ASDCP::MXF::WaveAudioDescriptor and the classes it derives
   from, so a caller can repeat the whole descriptor in an IMF CPL
   EssenceDescriptorList. Each has_* flag guards the field below it; the items
   without a flag are mandatory in asdcplib and always written. */
typedef struct {
    uint8_t instance_id[16];
    int32_t has_generation_id;
    uint8_t generation_id[16];

    uint32_t locator_count;
    uint8_t locators[ASDCP_MAX_LOCATORS][16];
    uint32_t sub_descriptor_count;
    uint8_t sub_descriptors[ASDCP_MAX_SOUND_SUB_DESCRIPTORS][16];

    int32_t has_linked_track_id;
    uint32_t linked_track_id;
    asdcp_rational_t sample_rate;
    int32_t has_container_duration;
    uint64_t container_duration;
    uint8_t essence_container[16];
    int32_t has_codec;
    uint8_t codec[16];

    asdcp_rational_t audio_sampling_rate;
    int32_t locked;
    int32_t has_audio_ref_level;
    uint8_t audio_ref_level;
    int32_t has_electro_spatial_formulation;
    uint8_t electro_spatial_formulation;
    uint32_t channel_count;
    uint32_t quantization_bits;
    int32_t has_dial_norm;
    uint8_t dial_norm;
    uint8_t sound_essence_coding[16];
    int32_t has_reference_audio_alignment_level;
    uint8_t reference_audio_alignment_level;
    int32_t has_reference_image_edit_rate;
    asdcp_rational_t reference_image_edit_rate;

    uint16_t block_align;
    int32_t has_sequence_offset;
    uint8_t sequence_offset;
    uint32_t avg_bps;
    int32_t has_channel_assignment;
    uint8_t channel_assignment[16];
} asdcp_wave_audio_descriptor_t;

/* asdcplib refuses to write an MCA string longer than 128 bytes, so 128 plus a
   terminator holds anything it wrote. */
#define ASDCP_MCA_STRING_CAPACITY 129

/* One SMPTE 377-4 MCA label subdescriptor. kind is 0 = audio channel,
   1 = soundfield group, 2 = group of soundfield groups. Strings are
   NUL-terminated and truncated to fit. Each has_* flag guards the field below
   it; instance_id, tag_symbol, label_dictionary_id and link_id are always
   present. */
typedef struct {
    uint8_t instance_id[16];
    int32_t kind;
    char tag_symbol[ASDCP_MCA_STRING_CAPACITY];
    uint8_t label_dictionary_id[16];
    uint8_t link_id[16];
    int32_t has_tag_name;
    char tag_name[ASDCP_MCA_STRING_CAPACITY];
    int32_t has_channel_id;
    uint32_t channel_id;
    int32_t has_spoken_language;
    char spoken_language[ASDCP_MCA_STRING_CAPACITY];
    int32_t has_soundfield_group_link_id;
    uint8_t soundfield_group_link_id[16];
    int32_t has_title;
    char title[ASDCP_MCA_STRING_CAPACITY];
    int32_t has_title_version;
    char title_version[ASDCP_MCA_STRING_CAPACITY];
    int32_t has_audio_content_kind;
    char audio_content_kind[ASDCP_MCA_STRING_CAPACITY];
    int32_t has_audio_element_kind;
    char audio_element_kind[ASDCP_MCA_STRING_CAPACITY];
} asdcp_mca_label_t;

/* The MCALabelSubDescriptor items ST 2067-2 section 5.3.6.5 requires on an IMF
   SoundfieldGroupLabelSubDescriptor. language is the RFC 5646 code every label
   the config produces carries; the other four are free text drawn from no
   registry. All five must be non-empty. */
typedef struct {
    const char* language;
    const char* title;
    const char* title_version;
    const char* audio_content_kind;
    const char* audio_element_kind;
} asdcp_soundfield_group_properties_t;

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
typedef void* asdcp_as02_jp2k_writer_t;
typedef void* asdcp_as02_jp2k_reader_t;
typedef void* asdcp_as02_pcm_writer_t;
typedef void* asdcp_as02_pcm_reader_t;
typedef void* asdcp_as02_timed_text_writer_t;
typedef void* asdcp_as02_timed_text_reader_t;
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
/* Open a JP2K MXF and set the picture essence descriptor's TransferCharacteristic
   UL (e.g. the SMPTE ST 2084 PQ UL for HDR). transfer_characteristic_ul is 16
   bytes, or null to leave it unset. */
asdcp_result_t asdcp_jp2k_writer_open_write_transfer(asdcp_jp2k_writer_t w, const char* filename,
    const asdcp_writer_info_t* info, const asdcp_picture_descriptor_t* desc,
    const uint8_t* transfer_characteristic_ul, uint32_t header_size);
/* Open a JP2K MXF and set HDR/WCG picture metadata (transfer characteristic, color
   primaries, ST 2086 mastering display). */
asdcp_result_t asdcp_jp2k_writer_open_write_hdr(asdcp_jp2k_writer_t w, const char* filename,
    const asdcp_writer_info_t* info, const asdcp_picture_descriptor_t* desc,
    const asdcp_hdr_metadata_t* hdr, uint32_t header_size);

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
/* Read the picture essence descriptor's TransferCharacteristic. When present,
   out_ul receives the 16-byte UL and has_transfer_characteristic is set to 1,
   otherwise it is 0. */
asdcp_result_t asdcp_jp2k_reader_read_transfer_characteristic(asdcp_jp2k_reader_t r,
    uint8_t* out_ul, int32_t* has_transfer_characteristic);
/* Read all HDR/WCG picture metadata off the descriptor. */
asdcp_result_t asdcp_jp2k_reader_read_hdr(asdcp_jp2k_reader_t r, asdcp_hdr_metadata_t* hdr);

/* PCM Writer */
asdcp_pcm_writer_t asdcp_pcm_writer_new(void);
void asdcp_pcm_writer_free(asdcp_pcm_writer_t w);
asdcp_result_t asdcp_pcm_writer_open_write(asdcp_pcm_writer_t w, const char* filename,
    const asdcp_writer_info_t* info, const asdcp_audio_descriptor_t* desc, uint32_t header_size);
asdcp_result_t asdcp_pcm_writer_write_frame(asdcp_pcm_writer_t w,
    const uint8_t* frame_data, uint32_t frame_size,
    asdcp_aes_enc_context_t enc_ctx, asdcp_hmac_context_t hmac_ctx);
asdcp_result_t asdcp_pcm_writer_finalize(asdcp_pcm_writer_t w);

/* Open a PCM MXF and attach SMPTE 377-4 MCA label subdescriptors. mca_config is
   an asdcp-wrap style config string, e.g. "51(L,R,C,LFE,Ls,Rs),HI,VIN".
   mca_language is the RFC 5646 tag every label carries, NULL or empty for
   asdcplib's own default. */
asdcp_result_t asdcp_pcm_writer_open_write_mca(asdcp_pcm_writer_t w, const char* filename,
    const asdcp_writer_info_t* info, const asdcp_audio_descriptor_t* desc,
    const char* mca_config, const char* mca_language, uint32_t header_size);

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
asdcp_result_t asdcp_pcm_reader_read_mca_labels(asdcp_pcm_reader_t r,
    uint32_t* channel_label_count, uint32_t* soundfield_group_count,
    int32_t* has_mca_channel_assignment);
/* Number of MCA label subdescriptors the WaveAudioDescriptor links. */
asdcp_result_t asdcp_pcm_reader_mca_label_count(asdcp_pcm_reader_t r, uint32_t* out_count);
/* Copy the index-th MCA label subdescriptor into out_label, in the order the
   WaveAudioDescriptor links them. The caller owns out_label and the shim keeps
   no reference to it. */
asdcp_result_t asdcp_pcm_reader_mca_label_info(asdcp_pcm_reader_t r, uint32_t index,
    asdcp_mca_label_t* out_label);

/* TimedText Writer */
asdcp_timed_text_writer_t asdcp_timed_text_writer_new(void);
void asdcp_timed_text_writer_free(asdcp_timed_text_writer_t w);
asdcp_result_t asdcp_timed_text_writer_open_write(asdcp_timed_text_writer_t w, const char* filename,
    const asdcp_writer_info_t* info, const asdcp_timed_text_descriptor_t* desc, uint32_t header_size);
asdcp_result_t asdcp_timed_text_writer_write_timed_text_resource(asdcp_timed_text_writer_t w,
    const char* xml_doc,
    asdcp_aes_enc_context_t enc_ctx, asdcp_hmac_context_t hmac_ctx);
asdcp_result_t asdcp_timed_text_writer_write_ancillary_resource(asdcp_timed_text_writer_t w,
    const uint8_t* resource_data, uint32_t resource_size,
    const uint8_t* resource_uuid, const char* mime_type,
    asdcp_aes_enc_context_t enc_ctx, asdcp_hmac_context_t hmac_ctx);
asdcp_result_t asdcp_timed_text_writer_finalize(asdcp_timed_text_writer_t w);
/* Open a TimedText MXF for writing, declaring the ancillary resources (fonts,
   images) that will follow so the reader can enumerate them. resource_uuids is
   resource_count*16 bytes; resource_types is resource_count int32 values
   (0 = binary, 1 = PNG, 2 = OpenType font). WriteAncillaryResource calls must
   follow in the same order. */
asdcp_result_t asdcp_timed_text_writer_open_write_with_resources(asdcp_timed_text_writer_t w,
    const char* filename, const asdcp_writer_info_t* info, const asdcp_timed_text_descriptor_t* desc,
    const uint8_t* resource_uuids, const int32_t* resource_types, uint32_t resource_count,
    uint32_t header_size);

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
/* Number of ancillary resources (fonts, images) declared in the MXF header. */
asdcp_result_t asdcp_timed_text_reader_ancillary_resource_count(asdcp_timed_text_reader_t r,
    uint32_t* out_count);
/* UUID and MIME type (0 = binary, 1 = PNG, 2 = OpenType font) of the index-th
   ancillary resource. */
asdcp_result_t asdcp_timed_text_reader_ancillary_resource_info(asdcp_timed_text_reader_t r,
    uint32_t index, uint8_t* out_uuid, int32_t* out_type);
/* Read the ancillary resource identified by resource_uuid into buf. On a short
   buffer, out_size still reports the bytes read into the header's stream. */
asdcp_result_t asdcp_timed_text_reader_read_ancillary_resource(asdcp_timed_text_reader_t r,
    const uint8_t* resource_uuid, uint8_t* buf, uint32_t buf_capacity, uint32_t* out_size,
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

/* ---- AS-02 (IMF / ST 2067-5) ---- */

/* AS-02 JP2K Writer (frame-wrapped) */
asdcp_as02_jp2k_writer_t asdcp_as02_jp2k_writer_new(void);
void asdcp_as02_jp2k_writer_free(asdcp_as02_jp2k_writer_t w);
asdcp_result_t asdcp_as02_jp2k_writer_open_write(asdcp_as02_jp2k_writer_t w, const char* filename,
    const asdcp_writer_info_t* info, const asdcp_picture_descriptor_t* desc, uint32_t header_size);
/* Open an AS-02 JP2K MXF and set HDR/WCG picture metadata (transfer characteristic,
   color primaries, ST 2086 mastering display) on the RGBA essence descriptor. */
asdcp_result_t asdcp_as02_jp2k_writer_open_write_hdr(asdcp_as02_jp2k_writer_t w, const char* filename,
    const asdcp_writer_info_t* info, const asdcp_picture_descriptor_t* desc,
    const asdcp_hdr_metadata_t* hdr, uint32_t header_size);
asdcp_result_t asdcp_as02_jp2k_writer_write_frame(asdcp_as02_jp2k_writer_t w,
    const uint8_t* frame_data, uint32_t frame_size,
    asdcp_aes_enc_context_t enc_ctx, asdcp_hmac_context_t hmac_ctx);
asdcp_result_t asdcp_as02_jp2k_writer_finalize(asdcp_as02_jp2k_writer_t w);

/* AS-02 JP2K Reader */
asdcp_as02_jp2k_reader_t asdcp_as02_jp2k_reader_new(void);
void asdcp_as02_jp2k_reader_free(asdcp_as02_jp2k_reader_t r);
asdcp_result_t asdcp_as02_jp2k_reader_open_read(asdcp_as02_jp2k_reader_t r, const char* filename);
asdcp_result_t asdcp_as02_jp2k_reader_close(asdcp_as02_jp2k_reader_t r);
asdcp_result_t asdcp_as02_jp2k_reader_fill_picture_descriptor(asdcp_as02_jp2k_reader_t r, asdcp_picture_descriptor_t* desc);
/* Read the RGBA essence descriptor's PictureEssenceCoding, PixelLayout and
   component reference levels. */
asdcp_result_t asdcp_as02_jp2k_reader_read_rgba_descriptor(asdcp_as02_jp2k_reader_t r,
    asdcp_rgba_descriptor_t* out);
/* Read every item of the RGBA essence descriptor, enough to repeat it in an IMF
   CPL EssenceDescriptorList. */
asdcp_result_t asdcp_as02_jp2k_reader_read_rgba_essence_descriptor(asdcp_as02_jp2k_reader_t r,
    asdcp_rgba_essence_descriptor_t* out);
/* Read every item of the JPEG2000PictureSubDescriptor the RGBA essence
   descriptor links. */
asdcp_result_t asdcp_as02_jp2k_reader_read_jpeg2000_sub_descriptor(asdcp_as02_jp2k_reader_t r,
    asdcp_jpeg2000_sub_descriptor_t* out);
/* Read all HDR/WCG picture metadata off the AS-02 descriptor. */
asdcp_result_t asdcp_as02_jp2k_reader_read_hdr(asdcp_as02_jp2k_reader_t r, asdcp_hdr_metadata_t* hdr);
asdcp_result_t asdcp_as02_jp2k_reader_fill_writer_info(asdcp_as02_jp2k_reader_t r, asdcp_writer_info_t* info);
asdcp_result_t asdcp_as02_jp2k_reader_read_frame(asdcp_as02_jp2k_reader_t r, uint32_t frame_number,
    uint8_t* buf, uint32_t buf_capacity, uint32_t* out_size,
    asdcp_aes_dec_context_t dec_ctx, asdcp_hmac_context_t hmac_ctx);

/* AS-02 PCM Writer (clip-wrapped). edit_rate is taken from desc->edit_rate. */
asdcp_as02_pcm_writer_t asdcp_as02_pcm_writer_new(void);
void asdcp_as02_pcm_writer_free(asdcp_as02_pcm_writer_t w);
asdcp_result_t asdcp_as02_pcm_writer_open_write(asdcp_as02_pcm_writer_t w, const char* filename,
    const asdcp_writer_info_t* info, const asdcp_audio_descriptor_t* desc, uint32_t header_size);
/* Open an AS-02 PCM MXF with SMPTE 377-4 MCA label subdescriptors parsed from an
   as-02-wrap style config string, e.g. "ST(L,R)" or "51(L,R,C,LFE,Ls,Rs)", and
   set the IMF MCA ChannelAssignment UL. soundfield_group carries the language
   for every label and the four items that go on the one
   SoundfieldGroupLabelSubDescriptor the config produced. */
asdcp_result_t asdcp_as02_pcm_writer_open_write_mca(asdcp_as02_pcm_writer_t w, const char* filename,
    const asdcp_writer_info_t* info, const asdcp_audio_descriptor_t* desc,
    const char* mca_config,
    const asdcp_soundfield_group_properties_t* soundfield_group, uint32_t header_size);
asdcp_result_t asdcp_as02_pcm_writer_write_frame(asdcp_as02_pcm_writer_t w,
    const uint8_t* frame_data, uint32_t frame_size,
    asdcp_aes_enc_context_t enc_ctx, asdcp_hmac_context_t hmac_ctx);
asdcp_result_t asdcp_as02_pcm_writer_finalize(asdcp_as02_pcm_writer_t w);

/* AS-02 PCM Reader. edit_rate must match the value used to write. */
asdcp_as02_pcm_reader_t asdcp_as02_pcm_reader_new(void);
void asdcp_as02_pcm_reader_free(asdcp_as02_pcm_reader_t r);
asdcp_result_t asdcp_as02_pcm_reader_open_read(asdcp_as02_pcm_reader_t r, const char* filename,
    int32_t edit_rate_num, int32_t edit_rate_den);
asdcp_result_t asdcp_as02_pcm_reader_close(asdcp_as02_pcm_reader_t r);
asdcp_result_t asdcp_as02_pcm_reader_fill_audio_descriptor(asdcp_as02_pcm_reader_t r, asdcp_audio_descriptor_t* desc);
asdcp_result_t asdcp_as02_pcm_reader_fill_writer_info(asdcp_as02_pcm_reader_t r, asdcp_writer_info_t* info);
asdcp_result_t asdcp_as02_pcm_reader_read_frame(asdcp_as02_pcm_reader_t r, uint32_t frame_number,
    uint8_t* buf, uint32_t buf_capacity, uint32_t* out_size,
    asdcp_aes_dec_context_t dec_ctx, asdcp_hmac_context_t hmac_ctx);
/* The WaveAudioDescriptor's ChannelAssignment UL, present = 0 when the item is
   absent. */
asdcp_result_t asdcp_as02_pcm_reader_read_channel_assignment(asdcp_as02_pcm_reader_t r,
    uint8_t* out_ul, int32_t* present);
/* Every item of the WaveAudioDescriptor, enough to repeat it in an IMF CPL
   EssenceDescriptorList. Fails when the descriptor links more subdescriptors
   than out has room for, rather than returning a short list. */
asdcp_result_t asdcp_as02_pcm_reader_read_wave_audio_descriptor(asdcp_as02_pcm_reader_t r,
    asdcp_wave_audio_descriptor_t* out);
/* Number of MCA label subdescriptors the WaveAudioDescriptor links. */
asdcp_result_t asdcp_as02_pcm_reader_mca_label_count(asdcp_as02_pcm_reader_t r, uint32_t* out_count);
/* Copy the index-th MCA label subdescriptor into out_label, in the order the
   WaveAudioDescriptor links them. */
asdcp_result_t asdcp_as02_pcm_reader_mca_label_info(asdcp_as02_pcm_reader_t r, uint32_t index,
    asdcp_mca_label_t* out_label);

/* AS-02 TimedText Writer */
asdcp_as02_timed_text_writer_t asdcp_as02_timed_text_writer_new(void);
void asdcp_as02_timed_text_writer_free(asdcp_as02_timed_text_writer_t w);
asdcp_result_t asdcp_as02_timed_text_writer_open_write(asdcp_as02_timed_text_writer_t w, const char* filename,
    const asdcp_writer_info_t* info, const asdcp_timed_text_descriptor_t* desc, uint32_t header_size);
asdcp_result_t asdcp_as02_timed_text_writer_write_timed_text_resource(asdcp_as02_timed_text_writer_t w,
    const char* xml_doc,
    asdcp_aes_enc_context_t enc_ctx, asdcp_hmac_context_t hmac_ctx);
asdcp_result_t asdcp_as02_timed_text_writer_write_ancillary_resource(asdcp_as02_timed_text_writer_t w,
    const uint8_t* resource_data, uint32_t resource_size,
    const uint8_t* resource_uuid, const char* mime_type,
    asdcp_aes_enc_context_t enc_ctx, asdcp_hmac_context_t hmac_ctx);
asdcp_result_t asdcp_as02_timed_text_writer_finalize(asdcp_as02_timed_text_writer_t w);

/* AS-02 TimedText Reader */
asdcp_as02_timed_text_reader_t asdcp_as02_timed_text_reader_new(void);
void asdcp_as02_timed_text_reader_free(asdcp_as02_timed_text_reader_t r);
asdcp_result_t asdcp_as02_timed_text_reader_open_read(asdcp_as02_timed_text_reader_t r, const char* filename);
asdcp_result_t asdcp_as02_timed_text_reader_close(asdcp_as02_timed_text_reader_t r);
asdcp_result_t asdcp_as02_timed_text_reader_fill_descriptor(asdcp_as02_timed_text_reader_t r, asdcp_timed_text_descriptor_t* desc);
asdcp_result_t asdcp_as02_timed_text_reader_fill_writer_info(asdcp_as02_timed_text_reader_t r, asdcp_writer_info_t* info);
asdcp_result_t asdcp_as02_timed_text_reader_read_timed_text_resource(asdcp_as02_timed_text_reader_t r,
    uint8_t* buf, uint32_t buf_capacity, uint32_t* out_size,
    asdcp_aes_dec_context_t dec_ctx, asdcp_hmac_context_t hmac_ctx);

/* Utility */
int32_t asdcp_result_ok(asdcp_result_t result);

#ifdef __cplusplus
}
#endif

#endif /* ASDCP_SHIM_H */

//! Raw FFI bindings to asdcplib via C shim.
//!
//! This crate compiles asdcplib from source (via CMake) and provides
//! a thin C shim layer that Rust can call through FFI.
//!
//! # Safety
//!
//! All functions in this crate are unsafe. Use the safe `asdcplib` wrapper crate instead.
#![allow(non_camel_case_types)]

use std::os::raw::{c_char, c_int};

/// Opaque handle to a JP2K MXF writer.
pub enum AsdcpJp2kWriter {}
/// Opaque handle to a JP2K MXF reader.
pub enum AsdcpJp2kReader {}
/// Opaque handle to a PCM MXF writer.
pub enum AsdcpPcmWriter {}
/// Opaque handle to a PCM MXF reader.
pub enum AsdcpPcmReader {}
/// Opaque handle to a TimedText MXF writer.
pub enum AsdcpTimedTextWriter {}
/// Opaque handle to a TimedText MXF reader.
pub enum AsdcpTimedTextReader {}
/// Opaque handle to an Atmos MXF writer.
pub enum AsdcpAtmosWriter {}
/// Opaque handle to an Atmos MXF reader.
pub enum AsdcpAtmosReader {}
/// Opaque handle to a JP2K stereoscopic MXF writer.
pub enum AsdcpJp2kSWriter {}
/// Opaque handle to a JP2K stereoscopic MXF reader.
pub enum AsdcpJp2kSReader {}
/// Opaque handle to an AES encryption context.
pub enum AsdcpAesEncContext {}
/// Opaque handle to an AES decryption context.
pub enum AsdcpAesDecContext {}
/// Opaque handle to an HMAC context.
pub enum AsdcpHmacContext {}

/// Writer identification info (C-compatible struct).
#[repr(C)]
#[derive(Debug, Clone)]
pub struct AsdcpWriterInfo {
    pub product_uuid: [u8; 16],
    pub asset_uuid: [u8; 16],
    pub context_id: [u8; 16],
    pub cryptographic_key_id: [u8; 16],
    pub encrypted_essence: c_int,
    pub uses_hmac: c_int,
    pub label_set_type: c_int, // 0=Unknown, 1=Interop, 2=SMPTE
}

/// Rational number.
#[repr(C)]
#[derive(Debug, Clone, Copy, Default)]
pub struct AsdcpRational {
    pub numerator: i32,
    pub denominator: i32,
}

/// JP2K picture descriptor (C-compatible subset).
#[repr(C)]
#[derive(Debug, Clone)]
pub struct AsdcpPictureDescriptor {
    pub edit_rate: AsdcpRational,
    pub sample_rate: AsdcpRational,
    pub stored_width: u32,
    pub stored_height: u32,
    pub aspect_ratio: AsdcpRational,
    pub container_duration: u32,
    pub csize: u16,
}

/// PCM audio descriptor (C-compatible).
#[repr(C)]
#[derive(Debug, Clone)]
pub struct AsdcpAudioDescriptor {
    pub edit_rate: AsdcpRational,
    pub audio_sampling_rate: AsdcpRational,
    pub locked: u32,
    pub channel_count: u32,
    pub quantization_bits: u32,
    pub block_align: u32,
    pub avg_bps: u32,
    pub linked_track_id: u32,
    pub container_duration: u32,
    pub channel_format: c_int,
}

/// Timed text descriptor (C-compatible subset).
#[repr(C)]
#[derive(Debug, Clone)]
pub struct AsdcpTimedTextDescriptor {
    pub edit_rate: AsdcpRational,
    pub container_duration: u32,
    pub asset_id: [u8; 16],
}

/// Atmos descriptor (C-compatible).
#[repr(C)]
#[derive(Debug, Clone)]
pub struct AsdcpAtmosDescriptor {
    pub edit_rate: AsdcpRational,
    pub container_duration: u32,
    pub asset_id: [u8; 16],
    pub data_essence_coding: [u8; 16],
    pub first_frame: u32,
    pub max_channel_count: u16,
    pub max_object_count: u16,
    pub atmos_id: [u8; 16],
    pub atmos_version: u8,
}

/// Essence type enum (mirrors ASDCP::EssenceType_t).
pub type AsdcpEssenceType = c_int;
pub const ESS_UNKNOWN: AsdcpEssenceType = 0;
pub const ESS_MPEG2_VES: AsdcpEssenceType = 1;
pub const ESS_JPEG_2000: AsdcpEssenceType = 2;
pub const ESS_PCM_24B_48K: AsdcpEssenceType = 3;
pub const ESS_PCM_24B_96K: AsdcpEssenceType = 4;
pub const ESS_TIMED_TEXT: AsdcpEssenceType = 5;
pub const ESS_JPEG_2000_S: AsdcpEssenceType = 6;
pub const ESS_DCDATA_UNKNOWN: AsdcpEssenceType = 7;
pub const ESS_DCDATA_DOLBY_ATMOS: AsdcpEssenceType = 8;

/// Result type (negative = error).
pub type AsdcpResult = c_int;

unsafe extern "C" {
    // ---- Version ----
    pub fn asdcp_version() -> *const c_char;

    // ---- Essence type detection ----
    pub fn asdcp_essence_type(
        filename: *const c_char,
        out_type: *mut AsdcpEssenceType,
    ) -> AsdcpResult;
    pub fn asdcp_raw_essence_type(
        filename: *const c_char,
        out_type: *mut AsdcpEssenceType,
    ) -> AsdcpResult;

    // ---- AES Encryption Context ----
    pub fn asdcp_aes_enc_context_new() -> *mut AsdcpAesEncContext;
    pub fn asdcp_aes_enc_context_free(ctx: *mut AsdcpAesEncContext);
    pub fn asdcp_aes_enc_context_init_key(
        ctx: *mut AsdcpAesEncContext,
        key: *const u8,
    ) -> AsdcpResult;
    pub fn asdcp_aes_enc_context_set_ivec(
        ctx: *mut AsdcpAesEncContext,
        ivec: *const u8,
    ) -> AsdcpResult;

    // ---- AES Decryption Context ----
    pub fn asdcp_aes_dec_context_new() -> *mut AsdcpAesDecContext;
    pub fn asdcp_aes_dec_context_free(ctx: *mut AsdcpAesDecContext);
    pub fn asdcp_aes_dec_context_init_key(
        ctx: *mut AsdcpAesDecContext,
        key: *const u8,
    ) -> AsdcpResult;
    pub fn asdcp_aes_dec_context_set_ivec(
        ctx: *mut AsdcpAesDecContext,
        ivec: *const u8,
    ) -> AsdcpResult;

    // ---- HMAC Context ----
    pub fn asdcp_hmac_context_new() -> *mut AsdcpHmacContext;
    pub fn asdcp_hmac_context_free(ctx: *mut AsdcpHmacContext);
    pub fn asdcp_hmac_context_init_key(
        ctx: *mut AsdcpHmacContext,
        key: *const u8,
        label_set: c_int,
    ) -> AsdcpResult;

    // ---- JP2K Writer ----
    pub fn asdcp_jp2k_writer_new() -> *mut AsdcpJp2kWriter;
    pub fn asdcp_jp2k_writer_free(w: *mut AsdcpJp2kWriter);
    pub fn asdcp_jp2k_writer_open_write(
        w: *mut AsdcpJp2kWriter,
        filename: *const c_char,
        info: *const AsdcpWriterInfo,
        desc: *const AsdcpPictureDescriptor,
        header_size: u32,
    ) -> AsdcpResult;
    pub fn asdcp_jp2k_writer_write_frame(
        w: *mut AsdcpJp2kWriter,
        frame_data: *const u8,
        frame_size: u32,
        enc_ctx: *mut AsdcpAesEncContext,
        hmac_ctx: *mut AsdcpHmacContext,
    ) -> AsdcpResult;
    pub fn asdcp_jp2k_writer_finalize(w: *mut AsdcpJp2kWriter) -> AsdcpResult;

    // ---- JP2K Reader ----
    pub fn asdcp_jp2k_reader_new() -> *mut AsdcpJp2kReader;
    pub fn asdcp_jp2k_reader_free(r: *mut AsdcpJp2kReader);
    pub fn asdcp_jp2k_reader_open_read(
        r: *mut AsdcpJp2kReader,
        filename: *const c_char,
    ) -> AsdcpResult;
    pub fn asdcp_jp2k_reader_close(r: *mut AsdcpJp2kReader) -> AsdcpResult;
    pub fn asdcp_jp2k_reader_fill_picture_descriptor(
        r: *mut AsdcpJp2kReader,
        desc: *mut AsdcpPictureDescriptor,
    ) -> AsdcpResult;
    pub fn asdcp_jp2k_reader_fill_writer_info(
        r: *mut AsdcpJp2kReader,
        info: *mut AsdcpWriterInfo,
    ) -> AsdcpResult;
    pub fn asdcp_jp2k_reader_read_frame(
        r: *mut AsdcpJp2kReader,
        frame_number: u32,
        buf: *mut u8,
        buf_capacity: u32,
        out_size: *mut u32,
        dec_ctx: *mut AsdcpAesDecContext,
        hmac_ctx: *mut AsdcpHmacContext,
    ) -> AsdcpResult;

    // ---- PCM Writer ----
    pub fn asdcp_pcm_writer_new() -> *mut AsdcpPcmWriter;
    pub fn asdcp_pcm_writer_free(w: *mut AsdcpPcmWriter);
    pub fn asdcp_pcm_writer_open_write(
        w: *mut AsdcpPcmWriter,
        filename: *const c_char,
        info: *const AsdcpWriterInfo,
        desc: *const AsdcpAudioDescriptor,
        header_size: u32,
    ) -> AsdcpResult;
    pub fn asdcp_pcm_writer_write_frame(
        w: *mut AsdcpPcmWriter,
        frame_data: *const u8,
        frame_size: u32,
        enc_ctx: *mut AsdcpAesEncContext,
        hmac_ctx: *mut AsdcpHmacContext,
    ) -> AsdcpResult;
    pub fn asdcp_pcm_writer_finalize(w: *mut AsdcpPcmWriter) -> AsdcpResult;

    // ---- PCM Reader ----
    pub fn asdcp_pcm_reader_new() -> *mut AsdcpPcmReader;
    pub fn asdcp_pcm_reader_free(r: *mut AsdcpPcmReader);
    pub fn asdcp_pcm_reader_open_read(
        r: *mut AsdcpPcmReader,
        filename: *const c_char,
    ) -> AsdcpResult;
    pub fn asdcp_pcm_reader_close(r: *mut AsdcpPcmReader) -> AsdcpResult;
    pub fn asdcp_pcm_reader_fill_audio_descriptor(
        r: *mut AsdcpPcmReader,
        desc: *mut AsdcpAudioDescriptor,
    ) -> AsdcpResult;
    pub fn asdcp_pcm_reader_fill_writer_info(
        r: *mut AsdcpPcmReader,
        info: *mut AsdcpWriterInfo,
    ) -> AsdcpResult;
    pub fn asdcp_pcm_reader_read_frame(
        r: *mut AsdcpPcmReader,
        frame_number: u32,
        buf: *mut u8,
        buf_capacity: u32,
        out_size: *mut u32,
        dec_ctx: *mut AsdcpAesDecContext,
        hmac_ctx: *mut AsdcpHmacContext,
    ) -> AsdcpResult;

    // ---- TimedText Writer ----
    pub fn asdcp_timed_text_writer_new() -> *mut AsdcpTimedTextWriter;
    pub fn asdcp_timed_text_writer_free(w: *mut AsdcpTimedTextWriter);
    pub fn asdcp_timed_text_writer_open_write(
        w: *mut AsdcpTimedTextWriter,
        filename: *const c_char,
        info: *const AsdcpWriterInfo,
        desc: *const AsdcpTimedTextDescriptor,
        header_size: u32,
    ) -> AsdcpResult;
    pub fn asdcp_timed_text_writer_write_timed_text_resource(
        w: *mut AsdcpTimedTextWriter,
        xml_doc: *const c_char,
        xml_len: u32,
        enc_ctx: *mut AsdcpAesEncContext,
        hmac_ctx: *mut AsdcpHmacContext,
    ) -> AsdcpResult;
    pub fn asdcp_timed_text_writer_write_ancillary_resource(
        w: *mut AsdcpTimedTextWriter,
        resource_data: *const u8,
        resource_size: u32,
        resource_uuid: *const u8,
        mime_type: *const c_char,
        enc_ctx: *mut AsdcpAesEncContext,
        hmac_ctx: *mut AsdcpHmacContext,
    ) -> AsdcpResult;
    pub fn asdcp_timed_text_writer_finalize(w: *mut AsdcpTimedTextWriter) -> AsdcpResult;

    // ---- TimedText Reader ----
    pub fn asdcp_timed_text_reader_new() -> *mut AsdcpTimedTextReader;
    pub fn asdcp_timed_text_reader_free(r: *mut AsdcpTimedTextReader);
    pub fn asdcp_timed_text_reader_open_read(
        r: *mut AsdcpTimedTextReader,
        filename: *const c_char,
    ) -> AsdcpResult;
    pub fn asdcp_timed_text_reader_close(r: *mut AsdcpTimedTextReader) -> AsdcpResult;
    pub fn asdcp_timed_text_reader_fill_descriptor(
        r: *mut AsdcpTimedTextReader,
        desc: *mut AsdcpTimedTextDescriptor,
    ) -> AsdcpResult;
    pub fn asdcp_timed_text_reader_fill_writer_info(
        r: *mut AsdcpTimedTextReader,
        info: *mut AsdcpWriterInfo,
    ) -> AsdcpResult;
    pub fn asdcp_timed_text_reader_read_timed_text_resource(
        r: *mut AsdcpTimedTextReader,
        buf: *mut u8,
        buf_capacity: u32,
        out_size: *mut u32,
        dec_ctx: *mut AsdcpAesDecContext,
        hmac_ctx: *mut AsdcpHmacContext,
    ) -> AsdcpResult;

    // ---- Atmos Writer ----
    pub fn asdcp_atmos_writer_new() -> *mut AsdcpAtmosWriter;
    pub fn asdcp_atmos_writer_free(w: *mut AsdcpAtmosWriter);
    pub fn asdcp_atmos_writer_open_write(
        w: *mut AsdcpAtmosWriter,
        filename: *const c_char,
        info: *const AsdcpWriterInfo,
        desc: *const AsdcpAtmosDescriptor,
        header_size: u32,
    ) -> AsdcpResult;
    pub fn asdcp_atmos_writer_write_frame(
        w: *mut AsdcpAtmosWriter,
        frame_data: *const u8,
        frame_size: u32,
        enc_ctx: *mut AsdcpAesEncContext,
        hmac_ctx: *mut AsdcpHmacContext,
    ) -> AsdcpResult;
    pub fn asdcp_atmos_writer_finalize(w: *mut AsdcpAtmosWriter) -> AsdcpResult;

    // ---- Atmos Reader ----
    pub fn asdcp_atmos_reader_new() -> *mut AsdcpAtmosReader;
    pub fn asdcp_atmos_reader_free(r: *mut AsdcpAtmosReader);
    pub fn asdcp_atmos_reader_open_read(
        r: *mut AsdcpAtmosReader,
        filename: *const c_char,
    ) -> AsdcpResult;
    pub fn asdcp_atmos_reader_close(r: *mut AsdcpAtmosReader) -> AsdcpResult;
    pub fn asdcp_atmos_reader_fill_atmos_descriptor(
        r: *mut AsdcpAtmosReader,
        desc: *mut AsdcpAtmosDescriptor,
    ) -> AsdcpResult;
    pub fn asdcp_atmos_reader_fill_writer_info(
        r: *mut AsdcpAtmosReader,
        info: *mut AsdcpWriterInfo,
    ) -> AsdcpResult;
    pub fn asdcp_atmos_reader_read_frame(
        r: *mut AsdcpAtmosReader,
        frame_number: u32,
        buf: *mut u8,
        buf_capacity: u32,
        out_size: *mut u32,
        dec_ctx: *mut AsdcpAesDecContext,
        hmac_ctx: *mut AsdcpHmacContext,
    ) -> AsdcpResult;

    // ---- JP2K Stereoscopic Writer ----
    pub fn asdcp_jp2k_s_writer_new() -> *mut AsdcpJp2kSWriter;
    pub fn asdcp_jp2k_s_writer_free(w: *mut AsdcpJp2kSWriter);
    pub fn asdcp_jp2k_s_writer_open_write(
        w: *mut AsdcpJp2kSWriter,
        filename: *const c_char,
        info: *const AsdcpWriterInfo,
        desc: *const AsdcpPictureDescriptor,
        header_size: u32,
    ) -> AsdcpResult;
    pub fn asdcp_jp2k_s_writer_write_frame(
        w: *mut AsdcpJp2kSWriter,
        frame_data: *const u8,
        frame_size: u32,
        phase: c_int, // 0=Left, 1=Right
        enc_ctx: *mut AsdcpAesEncContext,
        hmac_ctx: *mut AsdcpHmacContext,
    ) -> AsdcpResult;
    pub fn asdcp_jp2k_s_writer_finalize(w: *mut AsdcpJp2kSWriter) -> AsdcpResult;

    // ---- JP2K Stereoscopic Reader ----
    pub fn asdcp_jp2k_s_reader_new() -> *mut AsdcpJp2kSReader;
    pub fn asdcp_jp2k_s_reader_free(r: *mut AsdcpJp2kSReader);
    pub fn asdcp_jp2k_s_reader_open_read(
        r: *mut AsdcpJp2kSReader,
        filename: *const c_char,
    ) -> AsdcpResult;
    pub fn asdcp_jp2k_s_reader_close(r: *mut AsdcpJp2kSReader) -> AsdcpResult;
    pub fn asdcp_jp2k_s_reader_fill_picture_descriptor(
        r: *mut AsdcpJp2kSReader,
        desc: *mut AsdcpPictureDescriptor,
    ) -> AsdcpResult;
    pub fn asdcp_jp2k_s_reader_fill_writer_info(
        r: *mut AsdcpJp2kSReader,
        info: *mut AsdcpWriterInfo,
    ) -> AsdcpResult;
    pub fn asdcp_jp2k_s_reader_read_frame(
        r: *mut AsdcpJp2kSReader,
        frame_number: u32,
        phase: c_int,
        buf: *mut u8,
        buf_capacity: u32,
        out_size: *mut u32,
        dec_ctx: *mut AsdcpAesDecContext,
        hmac_ctx: *mut AsdcpHmacContext,
    ) -> AsdcpResult;

    // ---- Utility ----
    pub fn asdcp_result_ok(result: AsdcpResult) -> c_int;
}

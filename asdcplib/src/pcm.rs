//! PCM audio MXF read/write support.

use crate::crypto::{AesDecContext, AesEncContext, HmacContext};
use crate::error::{self, Result};
use crate::{Rational, WriterInfo};
use std::ffi::CString;

/// PCM channel format.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ChannelFormat {
    None = 0,
    Cfg1 = 1, // 5.1
    Cfg2 = 2, // 6.1
    Cfg3 = 3, // 7.1 SDDS
    Cfg4 = 4, // Wild Track
    Cfg5 = 5, // 7.1 DS
    Cfg6 = 6, // MCA labels
}

/// PCM audio descriptor.
#[derive(Debug, Clone)]
pub struct AudioDescriptor {
    pub edit_rate: Rational,
    pub audio_sampling_rate: Rational,
    pub locked: bool,
    pub channel_count: u32,
    pub quantization_bits: u32,
    pub block_align: u32,
    pub avg_bps: u32,
    pub linked_track_id: u32,
    pub container_duration: u32,
    pub channel_format: ChannelFormat,
}

impl AudioDescriptor {
    fn to_ffi(&self) -> asdcplib_sys::AsdcpAudioDescriptor {
        asdcplib_sys::AsdcpAudioDescriptor {
            edit_rate: self.edit_rate.to_ffi(),
            audio_sampling_rate: self.audio_sampling_rate.to_ffi(),
            locked: if self.locked { 1 } else { 0 },
            channel_count: self.channel_count,
            quantization_bits: self.quantization_bits,
            block_align: self.block_align,
            avg_bps: self.avg_bps,
            linked_track_id: self.linked_track_id,
            container_duration: self.container_duration,
            channel_format: self.channel_format as i32,
        }
    }

    fn from_ffi(ffi: &asdcplib_sys::AsdcpAudioDescriptor) -> Self {
        Self {
            edit_rate: Rational::from_ffi(&ffi.edit_rate),
            audio_sampling_rate: Rational::from_ffi(&ffi.audio_sampling_rate),
            locked: ffi.locked != 0,
            channel_count: ffi.channel_count,
            quantization_bits: ffi.quantization_bits,
            block_align: ffi.block_align,
            avg_bps: ffi.avg_bps,
            linked_track_id: ffi.linked_track_id,
            container_duration: ffi.container_duration,
            channel_format: match ffi.channel_format {
                1 => ChannelFormat::Cfg1,
                2 => ChannelFormat::Cfg2,
                3 => ChannelFormat::Cfg3,
                4 => ChannelFormat::Cfg4,
                5 => ChannelFormat::Cfg5,
                6 => ChannelFormat::Cfg6,
                _ => ChannelFormat::None,
            },
        }
    }
}

/// PCM MXF writer.
pub struct MxfWriter {
    ptr: *mut asdcplib_sys::AsdcpPcmWriter,
}

unsafe impl Send for MxfWriter {}

impl Default for MxfWriter {
    fn default() -> Self {
        Self::new()
    }
}

impl MxfWriter {
    pub fn new() -> Self {
        Self {
            ptr: unsafe { asdcplib_sys::asdcp_pcm_writer_new() },
        }
    }

    pub fn open_write(
        &mut self,
        filename: &str,
        info: &WriterInfo,
        desc: &AudioDescriptor,
        header_size: u32,
    ) -> Result<()> {
        let cstr = CString::new(filename)
            .map_err(|_| crate::Error::InvalidArgument("null byte in filename"))?;
        let ffi_info = info.to_ffi();
        let ffi_desc = desc.to_ffi();
        error::check(unsafe {
            asdcplib_sys::asdcp_pcm_writer_open_write(
                self.ptr,
                cstr.as_ptr(),
                &ffi_info,
                &ffi_desc,
                header_size,
            )
        })
    }

    pub fn write_frame(
        &mut self,
        frame_data: &[u8],
        enc_ctx: Option<&mut AesEncContext>,
        hmac_ctx: Option<&mut HmacContext>,
    ) -> Result<()> {
        let enc_ptr = enc_ctx.map_or(std::ptr::null_mut(), |c| c.as_mut_ptr());
        let hmac_ptr = hmac_ctx.map_or(std::ptr::null_mut(), |c| c.as_mut_ptr());
        error::check(unsafe {
            asdcplib_sys::asdcp_pcm_writer_write_frame(
                self.ptr,
                frame_data.as_ptr(),
                frame_data.len() as u32,
                enc_ptr,
                hmac_ptr,
            )
        })
    }

    pub fn finalize(&mut self) -> Result<()> {
        error::check(unsafe { asdcplib_sys::asdcp_pcm_writer_finalize(self.ptr) })
    }
}

impl Drop for MxfWriter {
    fn drop(&mut self) {
        unsafe { asdcplib_sys::asdcp_pcm_writer_free(self.ptr) }
    }
}

/// PCM MXF reader.
pub struct MxfReader {
    ptr: *mut asdcplib_sys::AsdcpPcmReader,
}

unsafe impl Send for MxfReader {}

impl Default for MxfReader {
    fn default() -> Self {
        Self::new()
    }
}

impl MxfReader {
    pub fn new() -> Self {
        Self {
            ptr: unsafe { asdcplib_sys::asdcp_pcm_reader_new() },
        }
    }

    pub fn open_read(&mut self, filename: &str) -> Result<()> {
        let cstr = CString::new(filename)
            .map_err(|_| crate::Error::InvalidArgument("null byte in filename"))?;
        error::check(unsafe { asdcplib_sys::asdcp_pcm_reader_open_read(self.ptr, cstr.as_ptr()) })
    }

    pub fn close(&mut self) -> Result<()> {
        error::check(unsafe { asdcplib_sys::asdcp_pcm_reader_close(self.ptr) })
    }

    pub fn audio_descriptor(&mut self) -> Result<AudioDescriptor> {
        let mut ffi = unsafe { std::mem::zeroed::<asdcplib_sys::AsdcpAudioDescriptor>() };
        error::check(unsafe {
            asdcplib_sys::asdcp_pcm_reader_fill_audio_descriptor(self.ptr, &mut ffi)
        })?;
        Ok(AudioDescriptor::from_ffi(&ffi))
    }

    pub fn writer_info(&mut self) -> Result<WriterInfo> {
        let mut ffi = unsafe { std::mem::zeroed::<asdcplib_sys::AsdcpWriterInfo>() };
        error::check(unsafe {
            asdcplib_sys::asdcp_pcm_reader_fill_writer_info(self.ptr, &mut ffi)
        })?;
        Ok(WriterInfo::from_ffi(&ffi))
    }

    pub fn read_frame(
        &mut self,
        frame_number: u32,
        buf: &mut [u8],
        dec_ctx: Option<&mut AesDecContext>,
        hmac_ctx: Option<&mut HmacContext>,
    ) -> Result<usize> {
        let dec_ptr = dec_ctx.map_or(std::ptr::null_mut(), |c| c.as_mut_ptr());
        let hmac_ptr = hmac_ctx.map_or(std::ptr::null_mut(), |c| c.as_mut_ptr());
        let mut out_size: u32 = 0;
        error::check(unsafe {
            asdcplib_sys::asdcp_pcm_reader_read_frame(
                self.ptr,
                frame_number,
                buf.as_mut_ptr(),
                buf.len() as u32,
                &mut out_size,
                dec_ptr,
                hmac_ptr,
            )
        })?;
        Ok(out_size as usize)
    }
}

impl Drop for MxfReader {
    fn drop(&mut self) {
        unsafe { asdcplib_sys::asdcp_pcm_reader_free(self.ptr) }
    }
}

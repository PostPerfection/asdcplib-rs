//! JPEG 2000 MXF read/write support.

use crate::crypto::{AesDecContext, AesEncContext, HmacContext};
use crate::error::{self, Result};
use crate::{Rational, WriterInfo};
use std::ffi::CString;

/// JPEG 2000 picture descriptor.
#[derive(Debug, Clone)]
pub struct PictureDescriptor {
    pub edit_rate: Rational,
    pub sample_rate: Rational,
    pub stored_width: u32,
    pub stored_height: u32,
    pub aspect_ratio: Rational,
    pub container_duration: u32,
    pub component_count: u16,
}

impl PictureDescriptor {
    fn to_ffi(&self) -> asdcplib_sys::AsdcpPictureDescriptor {
        asdcplib_sys::AsdcpPictureDescriptor {
            edit_rate: self.edit_rate.to_ffi(),
            sample_rate: self.sample_rate.to_ffi(),
            stored_width: self.stored_width,
            stored_height: self.stored_height,
            aspect_ratio: self.aspect_ratio.to_ffi(),
            container_duration: self.container_duration,
            csize: self.component_count,
        }
    }

    fn from_ffi(ffi: &asdcplib_sys::AsdcpPictureDescriptor) -> Self {
        Self {
            edit_rate: Rational::from_ffi(&ffi.edit_rate),
            sample_rate: Rational::from_ffi(&ffi.sample_rate),
            stored_width: ffi.stored_width,
            stored_height: ffi.stored_height,
            aspect_ratio: Rational::from_ffi(&ffi.aspect_ratio),
            container_duration: ffi.container_duration,
            component_count: ffi.csize,
        }
    }
}

/// JPEG 2000 MXF writer.
pub struct MxfWriter {
    ptr: *mut asdcplib_sys::AsdcpJp2kWriter,
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
            ptr: unsafe { asdcplib_sys::asdcp_jp2k_writer_new() },
        }
    }

    pub fn open_write(
        &mut self,
        filename: &str,
        info: &WriterInfo,
        desc: &PictureDescriptor,
        header_size: u32,
    ) -> Result<()> {
        let cstr = CString::new(filename)
            .map_err(|_| crate::Error::InvalidArgument("null byte in filename"))?;
        let ffi_info = info.to_ffi();
        let ffi_desc = desc.to_ffi();
        let result = unsafe {
            asdcplib_sys::asdcp_jp2k_writer_open_write(
                self.ptr,
                cstr.as_ptr(),
                &ffi_info,
                &ffi_desc,
                header_size,
            )
        };
        error::check(result)
    }

    pub fn write_frame(
        &mut self,
        frame_data: &[u8],
        enc_ctx: Option<&mut AesEncContext>,
        hmac_ctx: Option<&mut HmacContext>,
    ) -> Result<()> {
        let enc_ptr = enc_ctx.map_or(std::ptr::null_mut(), |c| c.as_mut_ptr());
        let hmac_ptr = hmac_ctx.map_or(std::ptr::null_mut(), |c| c.as_mut_ptr());
        let result = unsafe {
            asdcplib_sys::asdcp_jp2k_writer_write_frame(
                self.ptr,
                frame_data.as_ptr(),
                frame_data.len() as u32,
                enc_ptr,
                hmac_ptr,
            )
        };
        error::check(result)
    }

    pub fn finalize(&mut self) -> Result<()> {
        error::check(unsafe { asdcplib_sys::asdcp_jp2k_writer_finalize(self.ptr) })
    }
}

impl Drop for MxfWriter {
    fn drop(&mut self) {
        unsafe { asdcplib_sys::asdcp_jp2k_writer_free(self.ptr) }
    }
}

/// JPEG 2000 MXF reader.
pub struct MxfReader {
    ptr: *mut asdcplib_sys::AsdcpJp2kReader,
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
            ptr: unsafe { asdcplib_sys::asdcp_jp2k_reader_new() },
        }
    }

    pub fn open_read(&mut self, filename: &str) -> Result<()> {
        let cstr = CString::new(filename)
            .map_err(|_| crate::Error::InvalidArgument("null byte in filename"))?;
        error::check(unsafe { asdcplib_sys::asdcp_jp2k_reader_open_read(self.ptr, cstr.as_ptr()) })
    }

    pub fn close(&mut self) -> Result<()> {
        error::check(unsafe { asdcplib_sys::asdcp_jp2k_reader_close(self.ptr) })
    }

    pub fn picture_descriptor(&mut self) -> Result<PictureDescriptor> {
        let mut ffi = unsafe { std::mem::zeroed::<asdcplib_sys::AsdcpPictureDescriptor>() };
        let result =
            unsafe { asdcplib_sys::asdcp_jp2k_reader_fill_picture_descriptor(self.ptr, &mut ffi) };
        error::check(result)?;
        Ok(PictureDescriptor::from_ffi(&ffi))
    }

    pub fn writer_info(&mut self) -> Result<WriterInfo> {
        let mut ffi = unsafe { std::mem::zeroed::<asdcplib_sys::AsdcpWriterInfo>() };
        let result =
            unsafe { asdcplib_sys::asdcp_jp2k_reader_fill_writer_info(self.ptr, &mut ffi) };
        error::check(result)?;
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
        let result = unsafe {
            asdcplib_sys::asdcp_jp2k_reader_read_frame(
                self.ptr,
                frame_number,
                buf.as_mut_ptr(),
                buf.len() as u32,
                &mut out_size,
                dec_ptr,
                hmac_ptr,
            )
        };
        error::check(result)?;
        Ok(out_size as usize)
    }
}

impl Drop for MxfReader {
    fn drop(&mut self) {
        unsafe { asdcplib_sys::asdcp_jp2k_reader_free(self.ptr) }
    }
}

/// Stereoscopic phase.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum StereoscopicPhase {
    Left = 0,
    Right = 1,
}

/// Stereoscopic JP2K MXF writer.
pub struct StereoMxfWriter {
    ptr: *mut asdcplib_sys::AsdcpJp2kSWriter,
}

unsafe impl Send for StereoMxfWriter {}

impl Default for StereoMxfWriter {
    fn default() -> Self {
        Self::new()
    }
}

impl StereoMxfWriter {
    pub fn new() -> Self {
        Self {
            ptr: unsafe { asdcplib_sys::asdcp_jp2k_s_writer_new() },
        }
    }

    pub fn open_write(
        &mut self,
        filename: &str,
        info: &WriterInfo,
        desc: &PictureDescriptor,
        header_size: u32,
    ) -> Result<()> {
        let cstr = CString::new(filename)
            .map_err(|_| crate::Error::InvalidArgument("null byte in filename"))?;
        let ffi_info = info.to_ffi();
        let ffi_desc = desc.to_ffi();
        let result = unsafe {
            asdcplib_sys::asdcp_jp2k_s_writer_open_write(
                self.ptr,
                cstr.as_ptr(),
                &ffi_info,
                &ffi_desc,
                header_size,
            )
        };
        error::check(result)
    }

    pub fn write_frame(
        &mut self,
        frame_data: &[u8],
        phase: StereoscopicPhase,
        enc_ctx: Option<&mut AesEncContext>,
        hmac_ctx: Option<&mut HmacContext>,
    ) -> Result<()> {
        let enc_ptr = enc_ctx.map_or(std::ptr::null_mut(), |c| c.as_mut_ptr());
        let hmac_ptr = hmac_ctx.map_or(std::ptr::null_mut(), |c| c.as_mut_ptr());
        let result = unsafe {
            asdcplib_sys::asdcp_jp2k_s_writer_write_frame(
                self.ptr,
                frame_data.as_ptr(),
                frame_data.len() as u32,
                phase as i32,
                enc_ptr,
                hmac_ptr,
            )
        };
        error::check(result)
    }

    pub fn finalize(&mut self) -> Result<()> {
        error::check(unsafe { asdcplib_sys::asdcp_jp2k_s_writer_finalize(self.ptr) })
    }
}

impl Drop for StereoMxfWriter {
    fn drop(&mut self) {
        unsafe { asdcplib_sys::asdcp_jp2k_s_writer_free(self.ptr) }
    }
}

/// Stereoscopic JP2K MXF reader.
pub struct StereoMxfReader {
    ptr: *mut asdcplib_sys::AsdcpJp2kSReader,
}

unsafe impl Send for StereoMxfReader {}

impl Default for StereoMxfReader {
    fn default() -> Self {
        Self::new()
    }
}

impl StereoMxfReader {
    pub fn new() -> Self {
        Self {
            ptr: unsafe { asdcplib_sys::asdcp_jp2k_s_reader_new() },
        }
    }

    pub fn open_read(&mut self, filename: &str) -> Result<()> {
        let cstr = CString::new(filename)
            .map_err(|_| crate::Error::InvalidArgument("null byte in filename"))?;
        error::check(unsafe {
            asdcplib_sys::asdcp_jp2k_s_reader_open_read(self.ptr, cstr.as_ptr())
        })
    }

    pub fn close(&mut self) -> Result<()> {
        error::check(unsafe { asdcplib_sys::asdcp_jp2k_s_reader_close(self.ptr) })
    }

    pub fn picture_descriptor(&mut self) -> Result<PictureDescriptor> {
        let mut ffi = unsafe { std::mem::zeroed::<asdcplib_sys::AsdcpPictureDescriptor>() };
        let result = unsafe {
            asdcplib_sys::asdcp_jp2k_s_reader_fill_picture_descriptor(self.ptr, &mut ffi)
        };
        error::check(result)?;
        Ok(PictureDescriptor::from_ffi(&ffi))
    }

    pub fn writer_info(&mut self) -> Result<WriterInfo> {
        let mut ffi = unsafe { std::mem::zeroed::<asdcplib_sys::AsdcpWriterInfo>() };
        let result =
            unsafe { asdcplib_sys::asdcp_jp2k_s_reader_fill_writer_info(self.ptr, &mut ffi) };
        error::check(result)?;
        Ok(WriterInfo::from_ffi(&ffi))
    }

    pub fn read_frame(
        &mut self,
        frame_number: u32,
        phase: StereoscopicPhase,
        buf: &mut [u8],
        dec_ctx: Option<&mut AesDecContext>,
        hmac_ctx: Option<&mut HmacContext>,
    ) -> Result<usize> {
        let dec_ptr = dec_ctx.map_or(std::ptr::null_mut(), |c| c.as_mut_ptr());
        let hmac_ptr = hmac_ctx.map_or(std::ptr::null_mut(), |c| c.as_mut_ptr());
        let mut out_size: u32 = 0;
        let result = unsafe {
            asdcplib_sys::asdcp_jp2k_s_reader_read_frame(
                self.ptr,
                frame_number,
                phase as i32,
                buf.as_mut_ptr(),
                buf.len() as u32,
                &mut out_size,
                dec_ptr,
                hmac_ptr,
            )
        };
        error::check(result)?;
        Ok(out_size as usize)
    }
}

impl Drop for StereoMxfReader {
    fn drop(&mut self) {
        unsafe { asdcplib_sys::asdcp_jp2k_s_reader_free(self.ptr) }
    }
}

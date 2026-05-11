//! Timed Text (subtitle) MXF read/write support.

use crate::crypto::{AesDecContext, AesEncContext, HmacContext};
use crate::error::{self, Result};
use crate::{Rational, WriterInfo};
use std::ffi::CString;

/// Timed text descriptor.
#[derive(Debug, Clone)]
pub struct TimedTextDescriptor {
    pub edit_rate: Rational,
    pub container_duration: u32,
    pub asset_id: [u8; 16],
}

impl TimedTextDescriptor {
    fn to_ffi(&self) -> asdcplib_sys::AsdcpTimedTextDescriptor {
        asdcplib_sys::AsdcpTimedTextDescriptor {
            edit_rate: self.edit_rate.to_ffi(),
            container_duration: self.container_duration,
            asset_id: self.asset_id,
        }
    }

    fn from_ffi(ffi: &asdcplib_sys::AsdcpTimedTextDescriptor) -> Self {
        Self {
            edit_rate: Rational::from_ffi(&ffi.edit_rate),
            container_duration: ffi.container_duration,
            asset_id: ffi.asset_id,
        }
    }
}

/// Timed Text MXF writer.
pub struct MxfWriter {
    ptr: *mut asdcplib_sys::AsdcpTimedTextWriter,
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
            ptr: unsafe { asdcplib_sys::asdcp_timed_text_writer_new() },
        }
    }

    pub fn open_write(
        &mut self,
        filename: &str,
        info: &WriterInfo,
        desc: &TimedTextDescriptor,
        header_size: u32,
    ) -> Result<()> {
        let cstr =
            CString::new(filename).map_err(|_| crate::Error::InvalidArgument("null byte"))?;
        let ffi_info = info.to_ffi();
        let ffi_desc = desc.to_ffi();
        error::check(unsafe {
            asdcplib_sys::asdcp_timed_text_writer_open_write(
                self.ptr,
                cstr.as_ptr(),
                &ffi_info,
                &ffi_desc,
                header_size,
            )
        })
    }

    pub fn write_timed_text_resource(
        &mut self,
        xml: &str,
        enc_ctx: Option<&mut AesEncContext>,
        hmac_ctx: Option<&mut HmacContext>,
    ) -> Result<()> {
        let cstr =
            CString::new(xml).map_err(|_| crate::Error::InvalidArgument("null byte in XML"))?;
        let enc_ptr = enc_ctx.map_or(std::ptr::null_mut(), |c| c.as_mut_ptr());
        let hmac_ptr = hmac_ctx.map_or(std::ptr::null_mut(), |c| c.as_mut_ptr());
        error::check(unsafe {
            asdcplib_sys::asdcp_timed_text_writer_write_timed_text_resource(
                self.ptr,
                cstr.as_ptr(),
                xml.len() as u32,
                enc_ptr,
                hmac_ptr,
            )
        })
    }

    pub fn write_ancillary_resource(
        &mut self,
        data: &[u8],
        uuid: &[u8; 16],
        mime_type: &str,
        enc_ctx: Option<&mut AesEncContext>,
        hmac_ctx: Option<&mut HmacContext>,
    ) -> Result<()> {
        let mime_cstr =
            CString::new(mime_type).map_err(|_| crate::Error::InvalidArgument("null byte"))?;
        let enc_ptr = enc_ctx.map_or(std::ptr::null_mut(), |c| c.as_mut_ptr());
        let hmac_ptr = hmac_ctx.map_or(std::ptr::null_mut(), |c| c.as_mut_ptr());
        error::check(unsafe {
            asdcplib_sys::asdcp_timed_text_writer_write_ancillary_resource(
                self.ptr,
                data.as_ptr(),
                data.len() as u32,
                uuid.as_ptr(),
                mime_cstr.as_ptr(),
                enc_ptr,
                hmac_ptr,
            )
        })
    }

    pub fn finalize(&mut self) -> Result<()> {
        error::check(unsafe { asdcplib_sys::asdcp_timed_text_writer_finalize(self.ptr) })
    }
}

impl Drop for MxfWriter {
    fn drop(&mut self) {
        unsafe { asdcplib_sys::asdcp_timed_text_writer_free(self.ptr) }
    }
}

/// Timed Text MXF reader.
pub struct MxfReader {
    ptr: *mut asdcplib_sys::AsdcpTimedTextReader,
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
            ptr: unsafe { asdcplib_sys::asdcp_timed_text_reader_new() },
        }
    }

    pub fn open_read(&mut self, filename: &str) -> Result<()> {
        let cstr =
            CString::new(filename).map_err(|_| crate::Error::InvalidArgument("null byte"))?;
        error::check(unsafe {
            asdcplib_sys::asdcp_timed_text_reader_open_read(self.ptr, cstr.as_ptr())
        })
    }

    pub fn close(&mut self) -> Result<()> {
        error::check(unsafe { asdcplib_sys::asdcp_timed_text_reader_close(self.ptr) })
    }

    pub fn descriptor(&mut self) -> Result<TimedTextDescriptor> {
        let mut ffi = unsafe { std::mem::zeroed::<asdcplib_sys::AsdcpTimedTextDescriptor>() };
        error::check(unsafe {
            asdcplib_sys::asdcp_timed_text_reader_fill_descriptor(self.ptr, &mut ffi)
        })?;
        Ok(TimedTextDescriptor::from_ffi(&ffi))
    }

    pub fn writer_info(&mut self) -> Result<WriterInfo> {
        let mut ffi = unsafe { std::mem::zeroed::<asdcplib_sys::AsdcpWriterInfo>() };
        error::check(unsafe {
            asdcplib_sys::asdcp_timed_text_reader_fill_writer_info(self.ptr, &mut ffi)
        })?;
        Ok(WriterInfo::from_ffi(&ffi))
    }

    pub fn read_timed_text_resource(
        &mut self,
        buf: &mut [u8],
        dec_ctx: Option<&mut AesDecContext>,
        hmac_ctx: Option<&mut HmacContext>,
    ) -> Result<usize> {
        let dec_ptr = dec_ctx.map_or(std::ptr::null_mut(), |c| c.as_mut_ptr());
        let hmac_ptr = hmac_ctx.map_or(std::ptr::null_mut(), |c| c.as_mut_ptr());
        let mut out_size: u32 = 0;
        error::check(unsafe {
            asdcplib_sys::asdcp_timed_text_reader_read_timed_text_resource(
                self.ptr,
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
        unsafe { asdcplib_sys::asdcp_timed_text_reader_free(self.ptr) }
    }
}

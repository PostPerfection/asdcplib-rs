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
    pub(crate) fn to_ffi(&self) -> asdcplib_sys::AsdcpTimedTextDescriptor {
        asdcplib_sys::AsdcpTimedTextDescriptor {
            edit_rate: self.edit_rate.to_ffi(),
            container_duration: self.container_duration,
            asset_id: self.asset_id,
        }
    }

    pub(crate) fn from_ffi(ffi: &asdcplib_sys::AsdcpTimedTextDescriptor) -> Self {
        Self {
            edit_rate: Rational::from_ffi(&ffi.edit_rate),
            container_duration: ffi.container_duration,
            asset_id: ffi.asset_id,
        }
    }
}

/// MIME category of a timed-text ancillary resource.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MimeType {
    Binary = 0,
    Png = 1,
    OpenType = 2,
}

impl MimeType {
    fn from_i32(v: i32) -> Self {
        match v {
            1 => MimeType::Png,
            2 => MimeType::OpenType,
            _ => MimeType::Binary,
        }
    }
}

/// Identity of an ancillary resource (font, image) embedded in a timed-text MXF.
#[derive(Debug, Clone)]
pub struct AncillaryResourceInfo {
    pub uuid: [u8; 16],
    pub mime_type: MimeType,
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

    /// Open for writing, declaring the ancillary resources that will follow.
    /// The reader can only enumerate resources declared here, and the
    /// [`write_ancillary_resource`](Self::write_ancillary_resource) calls must
    /// follow in the same order.
    pub fn open_write_with_resources(
        &mut self,
        filename: &str,
        info: &WriterInfo,
        desc: &TimedTextDescriptor,
        resources: &[AncillaryResourceInfo],
        header_size: u32,
    ) -> Result<()> {
        let cstr =
            CString::new(filename).map_err(|_| crate::Error::InvalidArgument("null byte"))?;
        let ffi_info = info.to_ffi();
        let ffi_desc = desc.to_ffi();
        let mut uuids: Vec<u8> = Vec::with_capacity(resources.len() * 16);
        let mut types: Vec<i32> = Vec::with_capacity(resources.len());
        for r in resources {
            uuids.extend_from_slice(&r.uuid);
            types.push(r.mime_type as i32);
        }
        error::check(unsafe {
            asdcplib_sys::asdcp_timed_text_writer_open_write_with_resources(
                self.ptr,
                cstr.as_ptr(),
                &ffi_info,
                &ffi_desc,
                uuids.as_ptr(),
                types.as_ptr(),
                resources.len() as u32,
                header_size,
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
        let result = unsafe {
            asdcplib_sys::asdcp_timed_text_reader_read_timed_text_resource(
                self.ptr,
                buf.as_mut_ptr(),
                buf.len() as u32,
                &mut out_size,
                dec_ptr,
                hmac_ptr,
            )
        };
        // on a short buffer the shim leaves buf untouched and reports the size needed
        if result == asdcplib_sys::RESULT_SMALLBUF {
            return Err(crate::Error::BufferTooSmall {
                needed: out_size as usize,
                capacity: buf.len(),
            });
        }
        error::check(result)?;
        Ok(out_size as usize)
    }

    /// Number of ancillary resources (fonts, images) declared in the header.
    pub fn ancillary_resource_count(&mut self) -> Result<usize> {
        let mut count: u32 = 0;
        error::check(unsafe {
            asdcplib_sys::asdcp_timed_text_reader_ancillary_resource_count(self.ptr, &mut count)
        })?;
        Ok(count as usize)
    }

    /// UUID and MIME type of the `index`-th ancillary resource.
    pub fn ancillary_resource_info(&mut self, index: usize) -> Result<AncillaryResourceInfo> {
        let mut uuid = [0u8; 16];
        let mut ty: i32 = 0;
        error::check(unsafe {
            asdcplib_sys::asdcp_timed_text_reader_ancillary_resource_info(
                self.ptr,
                index as u32,
                uuid.as_mut_ptr(),
                &mut ty,
            )
        })?;
        Ok(AncillaryResourceInfo {
            uuid,
            mime_type: MimeType::from_i32(ty),
        })
    }

    /// Read the ancillary resource identified by `uuid` into `buf`, returning the
    /// byte count. Discover UUIDs via
    /// [`ancillary_resource_info`](Self::ancillary_resource_info). A too-small
    /// buffer yields [`Error::BufferTooSmall`](crate::Error::BufferTooSmall).
    pub fn read_ancillary_resource(
        &mut self,
        uuid: &[u8; 16],
        buf: &mut [u8],
        dec_ctx: Option<&mut AesDecContext>,
        hmac_ctx: Option<&mut HmacContext>,
    ) -> Result<usize> {
        let dec_ptr = dec_ctx.map_or(std::ptr::null_mut(), |c| c.as_mut_ptr());
        let hmac_ptr = hmac_ctx.map_or(std::ptr::null_mut(), |c| c.as_mut_ptr());
        let mut out_size: u32 = 0;
        let result = unsafe {
            asdcplib_sys::asdcp_timed_text_reader_read_ancillary_resource(
                self.ptr,
                uuid.as_ptr(),
                buf.as_mut_ptr(),
                buf.len() as u32,
                &mut out_size,
                dec_ptr,
                hmac_ptr,
            )
        };
        if result == asdcplib_sys::RESULT_SMALLBUF {
            return Err(crate::Error::BufferTooSmall {
                needed: out_size as usize,
                capacity: buf.len(),
            });
        }
        error::check(result)?;
        Ok(out_size as usize)
    }
}

impl Drop for MxfReader {
    fn drop(&mut self) {
        unsafe { asdcplib_sys::asdcp_timed_text_reader_free(self.ptr) }
    }
}

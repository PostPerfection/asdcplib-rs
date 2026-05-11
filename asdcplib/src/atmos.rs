//! Dolby Atmos MXF read/write support.

use crate::crypto::{AesDecContext, AesEncContext, HmacContext};
use crate::error::{self, Result};
use crate::{Rational, WriterInfo};
use std::ffi::CString;

/// Atmos descriptor.
#[derive(Debug, Clone)]
pub struct AtmosDescriptor {
    pub edit_rate: Rational,
    pub container_duration: u32,
    pub asset_id: [u8; 16],
    pub data_essence_coding: [u8; 16],
    pub first_frame: u32,
    pub max_channel_count: u16,
    pub max_object_count: u16,
    pub atmos_id: [u8; 16],
    pub atmos_version: u8,
}

impl AtmosDescriptor {
    fn to_ffi(&self) -> asdcplib_sys::AsdcpAtmosDescriptor {
        asdcplib_sys::AsdcpAtmosDescriptor {
            edit_rate: self.edit_rate.to_ffi(),
            container_duration: self.container_duration,
            asset_id: self.asset_id,
            data_essence_coding: self.data_essence_coding,
            first_frame: self.first_frame,
            max_channel_count: self.max_channel_count,
            max_object_count: self.max_object_count,
            atmos_id: self.atmos_id,
            atmos_version: self.atmos_version,
        }
    }

    fn from_ffi(ffi: &asdcplib_sys::AsdcpAtmosDescriptor) -> Self {
        Self {
            edit_rate: Rational::from_ffi(&ffi.edit_rate),
            container_duration: ffi.container_duration,
            asset_id: ffi.asset_id,
            data_essence_coding: ffi.data_essence_coding,
            first_frame: ffi.first_frame,
            max_channel_count: ffi.max_channel_count,
            max_object_count: ffi.max_object_count,
            atmos_id: ffi.atmos_id,
            atmos_version: ffi.atmos_version,
        }
    }
}

/// Atmos MXF writer.
pub struct MxfWriter {
    ptr: *mut asdcplib_sys::AsdcpAtmosWriter,
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
            ptr: unsafe { asdcplib_sys::asdcp_atmos_writer_new() },
        }
    }

    pub fn open_write(
        &mut self,
        filename: &str,
        info: &WriterInfo,
        desc: &AtmosDescriptor,
        header_size: u32,
    ) -> Result<()> {
        let cstr =
            CString::new(filename).map_err(|_| crate::Error::InvalidArgument("null byte"))?;
        let ffi_info = info.to_ffi();
        let ffi_desc = desc.to_ffi();
        error::check(unsafe {
            asdcplib_sys::asdcp_atmos_writer_open_write(
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
            asdcplib_sys::asdcp_atmos_writer_write_frame(
                self.ptr,
                frame_data.as_ptr(),
                frame_data.len() as u32,
                enc_ptr,
                hmac_ptr,
            )
        })
    }

    pub fn finalize(&mut self) -> Result<()> {
        error::check(unsafe { asdcplib_sys::asdcp_atmos_writer_finalize(self.ptr) })
    }
}

impl Drop for MxfWriter {
    fn drop(&mut self) {
        unsafe { asdcplib_sys::asdcp_atmos_writer_free(self.ptr) }
    }
}

/// Atmos MXF reader.
pub struct MxfReader {
    ptr: *mut asdcplib_sys::AsdcpAtmosReader,
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
            ptr: unsafe { asdcplib_sys::asdcp_atmos_reader_new() },
        }
    }

    pub fn open_read(&mut self, filename: &str) -> Result<()> {
        let cstr =
            CString::new(filename).map_err(|_| crate::Error::InvalidArgument("null byte"))?;
        error::check(unsafe { asdcplib_sys::asdcp_atmos_reader_open_read(self.ptr, cstr.as_ptr()) })
    }

    pub fn close(&mut self) -> Result<()> {
        error::check(unsafe { asdcplib_sys::asdcp_atmos_reader_close(self.ptr) })
    }

    pub fn atmos_descriptor(&mut self) -> Result<AtmosDescriptor> {
        let mut ffi = unsafe { std::mem::zeroed::<asdcplib_sys::AsdcpAtmosDescriptor>() };
        error::check(unsafe {
            asdcplib_sys::asdcp_atmos_reader_fill_atmos_descriptor(self.ptr, &mut ffi)
        })?;
        Ok(AtmosDescriptor::from_ffi(&ffi))
    }

    pub fn writer_info(&mut self) -> Result<WriterInfo> {
        let mut ffi = unsafe { std::mem::zeroed::<asdcplib_sys::AsdcpWriterInfo>() };
        error::check(unsafe {
            asdcplib_sys::asdcp_atmos_reader_fill_writer_info(self.ptr, &mut ffi)
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
            asdcplib_sys::asdcp_atmos_reader_read_frame(
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
        unsafe { asdcplib_sys::asdcp_atmos_reader_free(self.ptr) }
    }
}

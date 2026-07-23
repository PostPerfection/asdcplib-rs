//! JPEG 2000 MXF read/write support.

use crate::crypto::{AesDecContext, AesEncContext, HmacContext};
use crate::error::{self, Result};
use crate::{Rational, WriterInfo};
use std::ffi::CString;

/// SMPTE ST 2084 (PQ) transfer characteristic UL, for HDR picture essence.
/// Defined in asdcplib MDD.cpp as `TransferCharacteristic_SMPTEST2084`.
pub const TRANSFER_CHARACTERISTIC_ST2084: [u8; 16] = [
    0x06, 0x0e, 0x2b, 0x34, 0x04, 0x01, 0x01, 0x0d, 0x04, 0x01, 0x01, 0x01, 0x01, 0x0a, 0x00, 0x00,
];

/// ITU-R BT.2020 transfer characteristic UL (MDD.cpp `TransferCharacteristic_ITU2020`).
pub const TRANSFER_CHARACTERISTIC_BT2020: [u8; 16] = [
    0x06, 0x0e, 0x2b, 0x34, 0x04, 0x01, 0x01, 0x0e, 0x04, 0x01, 0x01, 0x01, 0x01, 0x09, 0x00, 0x00,
];

/// ITU-R BT.709 color primaries UL (MDD.cpp `ColorPrimaries_ITU709`).
pub const COLOR_PRIMARIES_BT709: [u8; 16] = [
    0x06, 0x0e, 0x2b, 0x34, 0x04, 0x01, 0x01, 0x06, 0x04, 0x01, 0x01, 0x01, 0x03, 0x03, 0x00, 0x00,
];

/// ITU-R BT.2020 color primaries UL (MDD.cpp `ColorPrimaries_ITU2020`).
pub const COLOR_PRIMARIES_BT2020: [u8; 16] = [
    0x06, 0x0e, 0x2b, 0x34, 0x04, 0x01, 0x01, 0x0d, 0x04, 0x01, 0x01, 0x01, 0x03, 0x04, 0x00, 0x00,
];

/// P3 D65 color primaries UL (MDD.cpp `ColorPrimaries_P3D65`).
pub const COLOR_PRIMARIES_P3D65: [u8; 16] = [
    0x06, 0x0e, 0x2b, 0x34, 0x04, 0x01, 0x01, 0x0d, 0x04, 0x01, 0x01, 0x01, 0x03, 0x06, 0x00, 0x00,
];

/// HDR/WCG picture metadata (SMPTE ST 2067-21). Every field is optional; only
/// those set are written. Chromaticity coordinates are raw ST 2086 u16 values
/// (0.00002 increments), luminance raw u32 (0.0001 cd/m^2 increments).
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct HdrMetadata {
    pub transfer_characteristic: Option<[u8; 16]>,
    pub color_primaries: Option<[u8; 16]>,
    /// ST 2086 display primaries as `[[x, y]; 3]` in First/Second/Third order.
    pub mastering_display_primaries: Option<[[u16; 2]; 3]>,
    pub mastering_display_white_point: Option<[u16; 2]>,
    pub mastering_display_max_luminance: Option<u32>,
    pub mastering_display_min_luminance: Option<u32>,
}

impl HdrMetadata {
    pub(crate) fn to_ffi(&self) -> asdcplib_sys::AsdcpHdrMetadata {
        let p = self.mastering_display_primaries.unwrap_or_default();
        asdcplib_sys::AsdcpHdrMetadata {
            has_transfer_characteristic: self.transfer_characteristic.is_some() as i32,
            transfer_characteristic: self.transfer_characteristic.unwrap_or_default(),
            has_color_primaries: self.color_primaries.is_some() as i32,
            color_primaries: self.color_primaries.unwrap_or_default(),
            has_mastering_display_primaries: self.mastering_display_primaries.is_some() as i32,
            mastering_display_primaries: [p[0][0], p[0][1], p[1][0], p[1][1], p[2][0], p[2][1]],
            has_mastering_display_white_point: self.mastering_display_white_point.is_some() as i32,
            mastering_display_white_point: self.mastering_display_white_point.unwrap_or_default(),
            has_mastering_display_max_luminance: self.mastering_display_max_luminance.is_some()
                as i32,
            mastering_display_max_luminance: self
                .mastering_display_max_luminance
                .unwrap_or_default(),
            has_mastering_display_min_luminance: self.mastering_display_min_luminance.is_some()
                as i32,
            mastering_display_min_luminance: self
                .mastering_display_min_luminance
                .unwrap_or_default(),
        }
    }

    pub(crate) fn from_ffi(ffi: &asdcplib_sys::AsdcpHdrMetadata) -> Self {
        let mp = &ffi.mastering_display_primaries;
        Self {
            transfer_characteristic: (ffi.has_transfer_characteristic != 0)
                .then_some(ffi.transfer_characteristic),
            color_primaries: (ffi.has_color_primaries != 0).then_some(ffi.color_primaries),
            mastering_display_primaries: (ffi.has_mastering_display_primaries != 0).then_some([
                [mp[0], mp[1]],
                [mp[2], mp[3]],
                [mp[4], mp[5]],
            ]),
            mastering_display_white_point: (ffi.has_mastering_display_white_point != 0)
                .then_some(ffi.mastering_display_white_point),
            mastering_display_max_luminance: (ffi.has_mastering_display_max_luminance != 0)
                .then_some(ffi.mastering_display_max_luminance),
            mastering_display_min_luminance: (ffi.has_mastering_display_min_luminance != 0)
                .then_some(ffi.mastering_display_min_luminance),
        }
    }
}

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
    pub(crate) fn to_ffi(&self) -> asdcplib_sys::AsdcpPictureDescriptor {
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

    pub(crate) fn from_ffi(ffi: &asdcplib_sys::AsdcpPictureDescriptor) -> Self {
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

    /// Open for writing and set the picture essence descriptor's
    /// TransferCharacteristic UL (e.g. [`TRANSFER_CHARACTERISTIC_ST2084`] for HDR).
    pub fn open_write_transfer(
        &mut self,
        filename: &str,
        info: &WriterInfo,
        desc: &PictureDescriptor,
        transfer_characteristic: &[u8; 16],
        header_size: u32,
    ) -> Result<()> {
        let cstr = CString::new(filename)
            .map_err(|_| crate::Error::InvalidArgument("null byte in filename"))?;
        let ffi_info = info.to_ffi();
        let ffi_desc = desc.to_ffi();
        let result = unsafe {
            asdcplib_sys::asdcp_jp2k_writer_open_write_transfer(
                self.ptr,
                cstr.as_ptr(),
                &ffi_info,
                &ffi_desc,
                transfer_characteristic.as_ptr(),
                header_size,
            )
        };
        error::check(result)
    }

    /// Open for writing and set HDR/WCG picture metadata (transfer characteristic,
    /// color primaries, ST 2086 mastering display) on the essence descriptor.
    pub fn open_write_hdr(
        &mut self,
        filename: &str,
        info: &WriterInfo,
        desc: &PictureDescriptor,
        hdr: &HdrMetadata,
        header_size: u32,
    ) -> Result<()> {
        let cstr = CString::new(filename)
            .map_err(|_| crate::Error::InvalidArgument("null byte in filename"))?;
        let ffi_info = info.to_ffi();
        let ffi_desc = desc.to_ffi();
        let ffi_hdr = hdr.to_ffi();
        let result = unsafe {
            asdcplib_sys::asdcp_jp2k_writer_open_write_hdr(
                self.ptr,
                cstr.as_ptr(),
                &ffi_info,
                &ffi_desc,
                &ffi_hdr,
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

    /// The picture essence descriptor's TransferCharacteristic UL, or `None`
    /// when the property is absent.
    pub fn transfer_characteristic(&mut self) -> Result<Option<[u8; 16]>> {
        let mut ul = [0u8; 16];
        let mut present: i32 = 0;
        error::check(unsafe {
            asdcplib_sys::asdcp_jp2k_reader_read_transfer_characteristic(
                self.ptr,
                ul.as_mut_ptr(),
                &mut present,
            )
        })?;
        Ok((present != 0).then_some(ul))
    }

    /// All HDR/WCG picture metadata present on the essence descriptor.
    pub fn hdr_metadata(&mut self) -> Result<HdrMetadata> {
        let mut ffi = unsafe { std::mem::zeroed::<asdcplib_sys::AsdcpHdrMetadata>() };
        error::check(unsafe { asdcplib_sys::asdcp_jp2k_reader_read_hdr(self.ptr, &mut ffi) })?;
        Ok(HdrMetadata::from_ffi(&ffi))
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

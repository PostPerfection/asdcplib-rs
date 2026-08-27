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

/// ITU-R BT.709 transfer characteristic UL (MDD.cpp `TransferCharacteristic_ITU709`),
/// the SDR transfer an IMF App 2E Rec.709 picture signals.
pub const TRANSFER_CHARACTERISTIC_BT709: [u8; 16] = [
    0x06, 0x0e, 0x2b, 0x34, 0x04, 0x01, 0x01, 0x01, 0x04, 0x01, 0x01, 0x01, 0x01, 0x02, 0x00, 0x00,
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

/// The picture essence coding labels an AS-02 writer picks from the
/// codestream's Rsize (MDD.cpp `JP2KEssenceCompression_*`). An IMF profile gets
/// the label for its main and sub level where MDD names one, otherwise the
/// generic label for its family below. Anything outside the cinema and IMF
/// profiles falls back to [`PICTURE_ESSENCE_CODING_BROADCAST_PROFILE_1`].
pub const PICTURE_ESSENCE_CODING_CINEMA_2K: [u8; 16] = [
    0x06, 0x0e, 0x2b, 0x34, 0x04, 0x01, 0x01, 0x09, 0x04, 0x01, 0x02, 0x02, 0x03, 0x01, 0x01, 0x03,
];

/// See [`PICTURE_ESSENCE_CODING_CINEMA_2K`].
pub const PICTURE_ESSENCE_CODING_CINEMA_4K: [u8; 16] = [
    0x06, 0x0e, 0x2b, 0x34, 0x04, 0x01, 0x01, 0x09, 0x04, 0x01, 0x02, 0x02, 0x03, 0x01, 0x01, 0x04,
];

/// See [`PICTURE_ESSENCE_CODING_CINEMA_2K`].
pub const PICTURE_ESSENCE_CODING_BROADCAST_PROFILE_1: [u8; 16] = [
    0x06, 0x0e, 0x2b, 0x34, 0x04, 0x01, 0x01, 0x0d, 0x04, 0x01, 0x02, 0x02, 0x03, 0x01, 0x01, 0x11,
];

/// See [`PICTURE_ESSENCE_CODING_CINEMA_2K`].
pub const PICTURE_ESSENCE_CODING_IMF_2K_LOSSY: [u8; 16] = [
    0x06, 0x0e, 0x2b, 0x34, 0x04, 0x01, 0x01, 0x0d, 0x04, 0x01, 0x02, 0x02, 0x03, 0x01, 0x02, 0x00,
];

/// See [`PICTURE_ESSENCE_CODING_CINEMA_2K`].
pub const PICTURE_ESSENCE_CODING_IMF_4K_LOSSY: [u8; 16] = [
    0x06, 0x0e, 0x2b, 0x34, 0x04, 0x01, 0x01, 0x0d, 0x04, 0x01, 0x02, 0x02, 0x03, 0x01, 0x03, 0x00,
];

/// See [`PICTURE_ESSENCE_CODING_CINEMA_2K`].
pub const PICTURE_ESSENCE_CODING_IMF_8K_LOSSY: [u8; 16] = [
    0x06, 0x0e, 0x2b, 0x34, 0x04, 0x01, 0x01, 0x0d, 0x04, 0x01, 0x02, 0x02, 0x03, 0x01, 0x04, 0x00,
];

/// See [`PICTURE_ESSENCE_CODING_CINEMA_2K`].
pub const PICTURE_ESSENCE_CODING_IMF_2K_REVERSIBLE: [u8; 16] = [
    0x06, 0x0e, 0x2b, 0x34, 0x04, 0x01, 0x01, 0x0d, 0x04, 0x01, 0x02, 0x02, 0x03, 0x01, 0x05, 0x00,
];

/// See [`PICTURE_ESSENCE_CODING_CINEMA_2K`].
pub const PICTURE_ESSENCE_CODING_IMF_4K_REVERSIBLE: [u8; 16] = [
    0x06, 0x0e, 0x2b, 0x34, 0x04, 0x01, 0x01, 0x0d, 0x04, 0x01, 0x02, 0x02, 0x03, 0x01, 0x06, 0x00,
];

/// IMF 4K lossy, main level 6 sub level 3: what Rsiz 0x0536 maps to, and what
/// Netflix's Sol Levante App 2E picture carries.
pub const PICTURE_ESSENCE_CODING_IMF_4K_LOSSY_6_3: [u8; 16] = [
    0x06, 0x0e, 0x2b, 0x34, 0x04, 0x01, 0x01, 0x0d, 0x04, 0x01, 0x02, 0x02, 0x03, 0x01, 0x03, 0x12,
];

/// See [`PICTURE_ESSENCE_CODING_CINEMA_2K`].
pub const PICTURE_ESSENCE_CODING_IMF_8K_REVERSIBLE: [u8; 16] = [
    0x06, 0x0e, 0x2b, 0x34, 0x04, 0x01, 0x01, 0x0d, 0x04, 0x01, 0x02, 0x02, 0x03, 0x01, 0x07, 0x00,
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

/// Number of precinct size bytes the COD marker can carry (ISO 15444-1 Annex
/// A.6.1), and the width of [`CodingStyleDefault::precinct_sizes`].
pub const MAX_PRECINCT_SIZES: usize = asdcplib_sys::ASDCP_JP2K_MAX_PRECINCT_SIZES;

/// Ssize packs the bit depth minus one in its low 7 bits and the signed flag in
/// the top bit.
const SSIZE_DEPTH_MASK: u8 = 0x7f;

/// One image component of the SIZ marker (ISO 15444-1 Annex A.5.1).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ImageComponent {
    /// Bit depth minus one in the low 7 bits, signed flag in the top bit.
    pub ssize: u8,
    /// Horizontal separation of the component samples on the reference grid.
    pub x_rsize: u8,
    /// Vertical separation of the component samples on the reference grid.
    pub y_rsize: u8,
}

impl ImageComponent {
    pub fn bit_depth(&self) -> u8 {
        (self.ssize & SSIZE_DEPTH_MASK) + 1
    }

    pub fn is_signed(&self) -> bool {
        self.ssize & !SSIZE_DEPTH_MASK != 0
    }
}

/// The COD marker segment (ISO 15444-1 Annex A.6.1).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CodingStyleDefault {
    pub scod: u8,
    pub progression_order: u8,
    pub number_of_layers: u16,
    pub multi_component_transform: u8,
    pub decomposition_levels: u8,
    pub codeblock_width: u8,
    pub codeblock_height: u8,
    pub codeblock_style: u8,
    pub transformation: u8,
    /// Trailing zeros mean the codestream signalled fewer precinct sizes.
    pub precinct_sizes: [u8; MAX_PRECINCT_SIZES],
}

/// The QCD marker segment (ISO 15444-1 Annex A.6.4).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct QuantizationDefault {
    pub sqcd: u8,
    /// Quantization step sizes, as many as the marker carried.
    pub spqcd: Vec<u8>,
}

/// The CAP marker segment (ISO 15444-1 Annex A.5.2).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ExtendedCapabilities {
    pub pcap: u32,
    pub ccap: Vec<u16>,
}

/// The JPEG 2000 codestream header values the MXF
/// `JPEG2000PictureSubDescriptor` carries: the SIZ image and tile grid, the
/// per-component depth and subsampling, and the COD, QCD and CAP marker
/// segments.
///
/// Only [`CodestreamHeader::parse`] builds one, so a picture descriptor cannot
/// reach the writer with a zeroed sub-descriptor.
#[derive(Debug, Clone, PartialEq, Eq)]
#[non_exhaustive]
pub struct CodestreamHeader {
    /// SIZ Rsize: the profile and level the codestream conforms to.
    pub rsize: u16,
    /// SIZ Xsize and Ysize: the reference grid size.
    pub xsize: u32,
    pub ysize: u32,
    /// SIZ XOsize and YOsize: the image offset within the reference grid.
    pub x_osize: u32,
    pub y_osize: u32,
    /// SIZ XTsize and YTsize: the tile size.
    pub xt_size: u32,
    pub yt_size: u32,
    /// SIZ XTOsize and YTOsize: the first tile's offset.
    pub xt_osize: u32,
    pub yt_osize: u32,
    pub components: Vec<ImageComponent>,
    pub coding_style_default: CodingStyleDefault,
    pub quantization_default: QuantizationDefault,
    /// `None` when the codestream carries no CAP marker.
    pub extended_capabilities: Option<ExtendedCapabilities>,
}

impl CodestreamHeader {
    /// Read the SIZ, COD, QCD and CAP markers of a JPEG 2000 codestream.
    pub fn parse(codestream: &[u8]) -> Result<Self> {
        let mut ffi = unsafe { std::mem::zeroed::<asdcplib_sys::AsdcpCodestreamHeader>() };
        error::check(unsafe {
            asdcplib_sys::asdcp_jp2k_parse_codestream_header(
                codestream.as_ptr(),
                codestream.len() as u32,
                &mut ffi,
            )
        })?;
        Ok(Self::from_ffi(&ffi))
    }

    fn from_ffi(ffi: &asdcplib_sys::AsdcpCodestreamHeader) -> Self {
        let components = ffi.image_components
            [..(ffi.csize as usize).min(asdcplib_sys::ASDCP_JP2K_MAX_COMPONENTS)]
            .iter()
            .map(|c| ImageComponent {
                ssize: c.ssize,
                x_rsize: c.x_rsize,
                y_rsize: c.y_rsize,
            })
            .collect();

        let cod = &ffi.coding_style_default;
        let quantization = &ffi.quantization_default;
        let capabilities = &ffi.extended_capabilities;

        Self {
            rsize: ffi.rsize,
            xsize: ffi.xsize,
            ysize: ffi.ysize,
            x_osize: ffi.x_osize,
            y_osize: ffi.y_osize,
            xt_size: ffi.xt_size,
            yt_size: ffi.yt_size,
            xt_osize: ffi.xt_osize,
            yt_osize: ffi.yt_osize,
            components,
            coding_style_default: CodingStyleDefault {
                scod: cod.scod,
                progression_order: cod.progression_order,
                number_of_layers: u16::from_be_bytes(cod.number_of_layers),
                multi_component_transform: cod.multi_component_transform,
                decomposition_levels: cod.decomposition_levels,
                codeblock_width: cod.codeblock_width,
                codeblock_height: cod.codeblock_height,
                codeblock_style: cod.codeblock_style,
                transformation: cod.transformation,
                precinct_sizes: cod.precinct_sizes,
            },
            quantization_default: QuantizationDefault {
                sqcd: quantization.sqcd,
                spqcd: quantization.spqcd[..quantization.spqcd_length as usize].to_vec(),
            },
            extended_capabilities: (capabilities.capability_count
                != asdcplib_sys::ASDCP_JP2K_NO_EXTENDED_CAPABILITIES)
                .then(|| ExtendedCapabilities {
                    pcap: capabilities.pcap,
                    ccap: capabilities.ccap[..capabilities.capability_count as usize].to_vec(),
                }),
        }
    }

    fn to_ffi(&self) -> asdcplib_sys::AsdcpCodestreamHeader {
        let mut ffi = unsafe { std::mem::zeroed::<asdcplib_sys::AsdcpCodestreamHeader>() };
        ffi.rsize = self.rsize;
        ffi.xsize = self.xsize;
        ffi.ysize = self.ysize;
        ffi.x_osize = self.x_osize;
        ffi.y_osize = self.y_osize;
        ffi.xt_size = self.xt_size;
        ffi.yt_size = self.yt_size;
        ffi.xt_osize = self.xt_osize;
        ffi.yt_osize = self.yt_osize;
        ffi.csize = self.components.len() as u16;

        for (slot, component) in ffi.image_components.iter_mut().zip(&self.components) {
            slot.ssize = component.ssize;
            slot.x_rsize = component.x_rsize;
            slot.y_rsize = component.y_rsize;
        }

        let cod = &self.coding_style_default;
        ffi.coding_style_default = asdcplib_sys::AsdcpCodingStyleDefault {
            scod: cod.scod,
            progression_order: cod.progression_order,
            number_of_layers: cod.number_of_layers.to_be_bytes(),
            multi_component_transform: cod.multi_component_transform,
            decomposition_levels: cod.decomposition_levels,
            codeblock_width: cod.codeblock_width,
            codeblock_height: cod.codeblock_height,
            codeblock_style: cod.codeblock_style,
            transformation: cod.transformation,
            precinct_sizes: cod.precinct_sizes,
        };

        let spqcd = &self.quantization_default.spqcd;
        ffi.quantization_default.sqcd = self.quantization_default.sqcd;
        ffi.quantization_default.spqcd[..spqcd.len()].copy_from_slice(spqcd);
        ffi.quantization_default.spqcd_length = spqcd.len() as u8;

        match &self.extended_capabilities {
            Some(capabilities) => {
                ffi.extended_capabilities.pcap = capabilities.pcap;
                ffi.extended_capabilities.capability_count = capabilities.ccap.len() as i8;
                ffi.extended_capabilities.ccap[..capabilities.ccap.len()]
                    .copy_from_slice(&capabilities.ccap);
            }
            None => {
                ffi.extended_capabilities.capability_count =
                    asdcplib_sys::ASDCP_JP2K_NO_EXTENDED_CAPABILITIES;
            }
        }
        ffi
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
    /// Parsed from the essence's first frame, so the MXF sub-descriptor
    /// describes the codestream that is actually wrapped.
    pub codestream: CodestreamHeader,
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
            codestream: self.codestream.to_ffi(),
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
            codestream: CodestreamHeader::from_ffi(&ffi.codestream),
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

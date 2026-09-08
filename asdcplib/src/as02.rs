//! AS-02 (IMF, SMPTE ST 2067-5) MXF read/write support.
//!
//! AS-02 is the IMF Essence Component wrapping mandated by IMF (ST 2067). It
//! differs from AS-DCP: JP2K is frame-wrapped with a distributed index, PCM is
//! clip-wrapped (so the reader needs the edit rate to slice the clip into
//! frames), and encryption is not available for the clip-wrapped PCM path.
//!
//! Descriptors are shared with the AS-DCP modules ([`crate::jp2k::PictureDescriptor`],
//! [`crate::pcm::AudioDescriptor`], [`crate::timed_text::TimedTextDescriptor`]).

/// AS-02 JPEG 2000 (frame-wrapped) read/write.
pub mod jp2k {
    use crate::crypto::{AesDecContext, AesEncContext, HmacContext};
    use crate::error::{self, Result};
    use crate::jp2k::{ExtendedCapabilities, HdrMetadata, PictureDescriptor};
    use crate::{Rational, WriterInfo};
    use std::ffi::CString;

    /// The RGBA essence descriptor properties an IMF picture track carries
    /// beyond the shared [`PictureDescriptor`]. The writer derives all of them
    /// from the codestream header.
    #[derive(Debug, Clone, PartialEq, Eq)]
    pub struct RgbaDescriptor {
        /// The profile label, e.g. [`crate::jp2k::PICTURE_ESSENCE_CODING_IMF_4K_LOSSY`].
        pub picture_essence_coding: Option<[u8; 16]>,
        /// SMPTE 377 pixel layout: component code and bit depth pairs,
        /// zero-terminated.
        pub pixel_layout: [u8; 16],
        pub component_max_ref: Option<u32>,
        pub component_min_ref: Option<u32>,
    }

    /// Every item of the MXF `RGBAEssenceDescriptor` an AS-02 JP2K picture track
    /// carries, including the ones its `FileDescriptor` and
    /// `GenericPictureEssenceDescriptor` bases define. An IMF CPL
    /// EssenceDescriptorList entry has to repeat all of them.
    #[derive(Debug, Clone, PartialEq, Eq)]
    pub struct RgbaEssenceDescriptor {
        pub instance_id: [u8; 16],
        pub generation_id: Option<[u8; 16]>,
        pub locators: Vec<[u8; 16]>,
        /// InstanceIDs of the sub-descriptors, the JPEG 2000 one among them.
        pub sub_descriptors: Vec<[u8; 16]>,
        pub linked_track_id: Option<u32>,
        pub sample_rate: Rational,
        /// ContainerDuration, which a CPL repeats as EssenceLength.
        pub container_duration: Option<u64>,
        /// EssenceContainer, which a CPL repeats as ContainerFormat.
        pub essence_container: [u8; 16],
        pub codec: Option<[u8; 16]>,
        pub signal_standard: Option<u8>,
        pub frame_layout: u8,
        pub stored_width: u32,
        pub stored_height: u32,
        pub stored_f2_offset: Option<u32>,
        pub sampled_width: Option<u32>,
        pub sampled_height: Option<u32>,
        pub sampled_x_offset: Option<u32>,
        pub sampled_y_offset: Option<u32>,
        pub display_height: Option<u32>,
        pub display_width: Option<u32>,
        pub display_x_offset: Option<u32>,
        pub display_y_offset: Option<u32>,
        pub display_f2_offset: Option<u32>,
        /// AspectRatio, which a CPL repeats as ImageAspectRatio.
        pub aspect_ratio: Rational,
        pub active_format_descriptor: Option<u8>,
        pub alpha_transparency: Option<u8>,
        pub image_alignment_offset: Option<u32>,
        pub image_start_offset: Option<u32>,
        pub image_end_offset: Option<u32>,
        pub field_dominance: Option<u8>,
        /// PictureEssenceCoding, which a CPL repeats as PictureCompression.
        pub picture_essence_coding: [u8; 16],
        pub coding_equations: Option<[u8; 16]>,
        pub alternative_center_cuts: Vec<[u8; 16]>,
        pub active_width: Option<u32>,
        pub active_height: Option<u32>,
        pub active_x_offset: Option<u32>,
        pub active_y_offset: Option<u32>,
        /// First and second line of the VideoLineMap pair.
        pub video_line_map: Option<[u32; 2]>,
        /// TransferCharacteristic, ColorPrimaries and the ST 2086 mastering
        /// display block, all of which sit on the same descriptor.
        pub hdr: HdrMetadata,
        pub component_max_ref: Option<u32>,
        pub component_min_ref: Option<u32>,
        pub alpha_min_ref: Option<u32>,
        pub alpha_max_ref: Option<u32>,
        pub scanning_direction: Option<u8>,
        /// SMPTE 377 component code and bit depth pairs, zero-terminated.
        pub pixel_layout: [u8; 16],
    }

    impl RgbaEssenceDescriptor {
        fn from_ffi(ffi: &asdcplib_sys::AsdcpRgbaEssenceDescriptor) -> Self {
            Self {
                instance_id: ffi.instance_id,
                generation_id: optional(ffi.has_generation_id, ffi.generation_id),
                locators: ffi.locators[..ffi.locator_count as usize].to_vec(),
                sub_descriptors: ffi.sub_descriptors[..ffi.sub_descriptor_count as usize].to_vec(),
                linked_track_id: optional(ffi.has_linked_track_id, ffi.linked_track_id),
                sample_rate: Rational::from_ffi(&ffi.sample_rate),
                container_duration: optional(ffi.has_container_duration, ffi.container_duration),
                essence_container: ffi.essence_container,
                codec: optional(ffi.has_codec, ffi.codec),
                signal_standard: optional(ffi.has_signal_standard, ffi.signal_standard),
                frame_layout: ffi.frame_layout,
                stored_width: ffi.stored_width,
                stored_height: ffi.stored_height,
                stored_f2_offset: optional(ffi.has_stored_f2_offset, ffi.stored_f2_offset),
                sampled_width: optional(ffi.has_sampled_width, ffi.sampled_width),
                sampled_height: optional(ffi.has_sampled_height, ffi.sampled_height),
                sampled_x_offset: optional(ffi.has_sampled_x_offset, ffi.sampled_x_offset),
                sampled_y_offset: optional(ffi.has_sampled_y_offset, ffi.sampled_y_offset),
                display_height: optional(ffi.has_display_height, ffi.display_height),
                display_width: optional(ffi.has_display_width, ffi.display_width),
                display_x_offset: optional(ffi.has_display_x_offset, ffi.display_x_offset),
                display_y_offset: optional(ffi.has_display_y_offset, ffi.display_y_offset),
                display_f2_offset: optional(ffi.has_display_f2_offset, ffi.display_f2_offset),
                aspect_ratio: Rational::from_ffi(&ffi.aspect_ratio),
                active_format_descriptor: optional(
                    ffi.has_active_format_descriptor,
                    ffi.active_format_descriptor,
                ),
                alpha_transparency: optional(ffi.has_alpha_transparency, ffi.alpha_transparency),
                image_alignment_offset: optional(
                    ffi.has_image_alignment_offset,
                    ffi.image_alignment_offset,
                ),
                image_start_offset: optional(ffi.has_image_start_offset, ffi.image_start_offset),
                image_end_offset: optional(ffi.has_image_end_offset, ffi.image_end_offset),
                field_dominance: optional(ffi.has_field_dominance, ffi.field_dominance),
                picture_essence_coding: ffi.picture_essence_coding,
                coding_equations: optional(ffi.has_coding_equations, ffi.coding_equations),
                alternative_center_cuts: ffi.alternative_center_cuts
                    [..ffi.alternative_center_cut_count as usize]
                    .to_vec(),
                active_width: optional(ffi.has_active_width, ffi.active_width),
                active_height: optional(ffi.has_active_height, ffi.active_height),
                active_x_offset: optional(ffi.has_active_x_offset, ffi.active_x_offset),
                active_y_offset: optional(ffi.has_active_y_offset, ffi.active_y_offset),
                video_line_map: optional(ffi.has_video_line_map, ffi.video_line_map),
                hdr: HdrMetadata::from_ffi(&ffi.hdr),
                component_max_ref: optional(ffi.has_component_max_ref, ffi.component_max_ref),
                component_min_ref: optional(ffi.has_component_min_ref, ffi.component_min_ref),
                alpha_min_ref: optional(ffi.has_alpha_min_ref, ffi.alpha_min_ref),
                alpha_max_ref: optional(ffi.has_alpha_max_ref, ffi.alpha_max_ref),
                scanning_direction: optional(ffi.has_scanning_direction, ffi.scanning_direction),
                pixel_layout: ffi.pixel_layout,
            }
        }
    }

    /// Every item of the MXF `JPEG2000PictureSubDescriptor` the RGBA essence
    /// descriptor links.
    #[derive(Debug, Clone, PartialEq, Eq)]
    pub struct Jpeg2000PictureSubDescriptor {
        pub instance_id: [u8; 16],
        pub generation_id: Option<[u8; 16]>,
        pub rsize: u16,
        pub xsize: u32,
        pub ysize: u32,
        pub x_osize: u32,
        pub y_osize: u32,
        pub xt_size: u32,
        pub yt_size: u32,
        pub xt_osize: u32,
        pub yt_osize: u32,
        pub csize: u16,
        /// The SIZ per-component Ssize, XRsize and YRsize triples, preceded by
        /// the big-endian item count and item size.
        pub picture_component_sizing: Option<Vec<u8>>,
        /// The COD marker segment bytes.
        pub coding_style_default: Option<Vec<u8>>,
        /// The QCD marker segment bytes.
        pub quantization_default: Option<Vec<u8>>,
        /// Component code and bit depth pairs, one per codestream component,
        /// zero-terminated. This is where Photon reads the pixel bit depth.
        pub j2c_layout: Option<[u8; 16]>,
        pub extended_capabilities: Option<ExtendedCapabilities>,
        pub profile: Option<Vec<u16>>,
        pub corresponding_profile: Option<Vec<u16>>,
    }

    impl Jpeg2000PictureSubDescriptor {
        fn from_ffi(ffi: &asdcplib_sys::AsdcpJpeg2000SubDescriptor) -> Self {
            Self {
                instance_id: ffi.instance_id,
                generation_id: optional(ffi.has_generation_id, ffi.generation_id),
                rsize: ffi.rsize,
                xsize: ffi.xsize,
                ysize: ffi.ysize,
                x_osize: ffi.x_osize,
                y_osize: ffi.y_osize,
                xt_size: ffi.xt_size,
                yt_size: ffi.yt_size,
                xt_osize: ffi.xt_osize,
                yt_osize: ffi.yt_osize,
                csize: ffi.csize,
                picture_component_sizing: optional(
                    ffi.has_picture_component_sizing,
                    ffi.picture_component_sizing[..ffi.picture_component_sizing_length as usize]
                        .to_vec(),
                ),
                coding_style_default: optional(
                    ffi.has_coding_style_default,
                    ffi.coding_style_default[..ffi.coding_style_default_length as usize].to_vec(),
                ),
                quantization_default: optional(
                    ffi.has_quantization_default,
                    ffi.quantization_default[..ffi.quantization_default_length as usize].to_vec(),
                ),
                j2c_layout: optional(ffi.has_j2c_layout, ffi.j2c_layout),
                extended_capabilities: optional(
                    ffi.has_extended_capabilities,
                    ExtendedCapabilities {
                        pcap: ffi.pcap,
                        ccap: ffi.ccap[..ffi.capability_count as usize].to_vec(),
                    },
                ),
                profile: optional(
                    ffi.has_profile,
                    ffi.profile[..ffi.profile_count as usize].to_vec(),
                ),
                corresponding_profile: optional(
                    ffi.has_corresponding_profile,
                    ffi.corresponding_profile[..ffi.corresponding_profile_count as usize].to_vec(),
                ),
            }
        }
    }

    fn optional<T>(has_value: i32, value: T) -> Option<T> {
        if has_value != 0 { Some(value) } else { None }
    }

    /// AS-02 JPEG 2000 MXF writer.
    pub struct MxfWriter {
        ptr: *mut asdcplib_sys::AsdcpAs02Jp2kWriter,
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
                ptr: unsafe { asdcplib_sys::asdcp_as02_jp2k_writer_new() },
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
            error::check(unsafe {
                asdcplib_sys::asdcp_as02_jp2k_writer_open_write(
                    self.ptr,
                    cstr.as_ptr(),
                    &ffi_info,
                    &ffi_desc,
                    header_size,
                )
            })
        }

        /// Open for writing and set HDR/WCG picture metadata (transfer
        /// characteristic, color primaries, ST 2086 mastering display) on the
        /// AS-02 RGBA essence descriptor.
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
            error::check(unsafe {
                asdcplib_sys::asdcp_as02_jp2k_writer_open_write_hdr(
                    self.ptr,
                    cstr.as_ptr(),
                    &ffi_info,
                    &ffi_desc,
                    &ffi_hdr,
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
                asdcplib_sys::asdcp_as02_jp2k_writer_write_frame(
                    self.ptr,
                    frame_data.as_ptr(),
                    frame_data.len() as u32,
                    enc_ptr,
                    hmac_ptr,
                )
            })
        }

        pub fn finalize(&mut self) -> Result<()> {
            error::check(unsafe { asdcplib_sys::asdcp_as02_jp2k_writer_finalize(self.ptr) })
        }
    }

    impl Drop for MxfWriter {
        fn drop(&mut self) {
            unsafe { asdcplib_sys::asdcp_as02_jp2k_writer_free(self.ptr) }
        }
    }

    /// AS-02 JPEG 2000 MXF reader.
    pub struct MxfReader {
        ptr: *mut asdcplib_sys::AsdcpAs02Jp2kReader,
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
                ptr: unsafe { asdcplib_sys::asdcp_as02_jp2k_reader_new() },
            }
        }

        pub fn open_read(&mut self, filename: &str) -> Result<()> {
            let cstr = CString::new(filename)
                .map_err(|_| crate::Error::InvalidArgument("null byte in filename"))?;
            error::check(unsafe {
                asdcplib_sys::asdcp_as02_jp2k_reader_open_read(self.ptr, cstr.as_ptr())
            })
        }

        pub fn close(&mut self) -> Result<()> {
            error::check(unsafe { asdcplib_sys::asdcp_as02_jp2k_reader_close(self.ptr) })
        }

        pub fn picture_descriptor(&mut self) -> Result<PictureDescriptor> {
            let mut ffi = unsafe { std::mem::zeroed::<asdcplib_sys::AsdcpPictureDescriptor>() };
            error::check(unsafe {
                asdcplib_sys::asdcp_as02_jp2k_reader_fill_picture_descriptor(self.ptr, &mut ffi)
            })?;
            Ok(PictureDescriptor::from_ffi(&ffi))
        }

        /// The RGBA essence descriptor's profile label, pixel layout and
        /// component reference levels.
        pub fn rgba_descriptor(&mut self) -> Result<RgbaDescriptor> {
            let mut ffi = unsafe { std::mem::zeroed::<asdcplib_sys::AsdcpRgbaDescriptor>() };
            error::check(unsafe {
                asdcplib_sys::asdcp_as02_jp2k_reader_read_rgba_descriptor(self.ptr, &mut ffi)
            })?;
            Ok(RgbaDescriptor {
                picture_essence_coding: (ffi.has_picture_essence_coding != 0)
                    .then_some(ffi.picture_essence_coding),
                pixel_layout: ffi.pixel_layout,
                component_max_ref: (ffi.has_component_max_ref != 0)
                    .then_some(ffi.component_max_ref),
                component_min_ref: (ffi.has_component_min_ref != 0)
                    .then_some(ffi.component_min_ref),
            })
        }

        /// Every item of the RGBA essence descriptor, enough to repeat it in an
        /// IMF CPL EssenceDescriptorList.
        pub fn rgba_essence_descriptor(&mut self) -> Result<RgbaEssenceDescriptor> {
            let mut ffi = unsafe { std::mem::zeroed::<asdcplib_sys::AsdcpRgbaEssenceDescriptor>() };
            error::check(unsafe {
                asdcplib_sys::asdcp_as02_jp2k_reader_read_rgba_essence_descriptor(
                    self.ptr, &mut ffi,
                )
            })?;
            Ok(RgbaEssenceDescriptor::from_ffi(&ffi))
        }

        /// Every item of the JPEG 2000 picture sub-descriptor the RGBA essence
        /// descriptor links.
        pub fn jpeg2000_sub_descriptor(&mut self) -> Result<Jpeg2000PictureSubDescriptor> {
            let mut ffi = unsafe { std::mem::zeroed::<asdcplib_sys::AsdcpJpeg2000SubDescriptor>() };
            error::check(unsafe {
                asdcplib_sys::asdcp_as02_jp2k_reader_read_jpeg2000_sub_descriptor(
                    self.ptr, &mut ffi,
                )
            })?;
            Ok(Jpeg2000PictureSubDescriptor::from_ffi(&ffi))
        }

        /// All HDR/WCG picture metadata present on the AS-02 essence descriptor.
        pub fn hdr_metadata(&mut self) -> Result<HdrMetadata> {
            let mut ffi = unsafe { std::mem::zeroed::<asdcplib_sys::AsdcpHdrMetadata>() };
            error::check(unsafe {
                asdcplib_sys::asdcp_as02_jp2k_reader_read_hdr(self.ptr, &mut ffi)
            })?;
            Ok(HdrMetadata::from_ffi(&ffi))
        }

        pub fn writer_info(&mut self) -> Result<WriterInfo> {
            let mut ffi = unsafe { std::mem::zeroed::<asdcplib_sys::AsdcpWriterInfo>() };
            error::check(unsafe {
                asdcplib_sys::asdcp_as02_jp2k_reader_fill_writer_info(self.ptr, &mut ffi)
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
                asdcplib_sys::asdcp_as02_jp2k_reader_read_frame(
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
            unsafe { asdcplib_sys::asdcp_as02_jp2k_reader_free(self.ptr) }
        }
    }
}

/// AS-02 PCM audio (clip-wrapped) read/write.
pub mod pcm {
    use crate::crypto::{AesDecContext, AesEncContext, HmacContext};
    use crate::error::{self, Result};
    use crate::pcm::AudioDescriptor;
    use crate::{Rational, WriterInfo};
    use std::ffi::CString;

    /// AS-02 PCM MXF writer.
    pub struct MxfWriter {
        ptr: *mut asdcplib_sys::AsdcpAs02PcmWriter,
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
                ptr: unsafe { asdcplib_sys::asdcp_as02_pcm_writer_new() },
            }
        }

        /// The edit rate is taken from `desc.edit_rate`. Encryption is not
        /// supported for clip-wrapped PCM, so `open_write` fails if
        /// `info.encrypted_essence` is set.
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
                asdcplib_sys::asdcp_as02_pcm_writer_open_write(
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
                asdcplib_sys::asdcp_as02_pcm_writer_write_frame(
                    self.ptr,
                    frame_data.as_ptr(),
                    frame_data.len() as u32,
                    enc_ptr,
                    hmac_ptr,
                )
            })
        }

        pub fn finalize(&mut self) -> Result<()> {
            error::check(unsafe { asdcplib_sys::asdcp_as02_pcm_writer_finalize(self.ptr) })
        }
    }

    impl Drop for MxfWriter {
        fn drop(&mut self) {
            unsafe { asdcplib_sys::asdcp_as02_pcm_writer_free(self.ptr) }
        }
    }

    /// AS-02 PCM MXF reader.
    pub struct MxfReader {
        ptr: *mut asdcplib_sys::AsdcpAs02PcmReader,
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
                ptr: unsafe { asdcplib_sys::asdcp_as02_pcm_reader_new() },
            }
        }

        /// `edit_rate` slices the clip into frames and must match the value used
        /// to write the file.
        pub fn open_read(&mut self, filename: &str, edit_rate: Rational) -> Result<()> {
            let cstr = CString::new(filename)
                .map_err(|_| crate::Error::InvalidArgument("null byte in filename"))?;
            error::check(unsafe {
                asdcplib_sys::asdcp_as02_pcm_reader_open_read(
                    self.ptr,
                    cstr.as_ptr(),
                    edit_rate.numerator,
                    edit_rate.denominator,
                )
            })
        }

        pub fn close(&mut self) -> Result<()> {
            error::check(unsafe { asdcplib_sys::asdcp_as02_pcm_reader_close(self.ptr) })
        }

        pub fn audio_descriptor(&mut self) -> Result<AudioDescriptor> {
            let mut ffi = unsafe { std::mem::zeroed::<asdcplib_sys::AsdcpAudioDescriptor>() };
            error::check(unsafe {
                asdcplib_sys::asdcp_as02_pcm_reader_fill_audio_descriptor(self.ptr, &mut ffi)
            })?;
            Ok(AudioDescriptor::from_ffi(&ffi))
        }

        pub fn writer_info(&mut self) -> Result<WriterInfo> {
            let mut ffi = unsafe { std::mem::zeroed::<asdcplib_sys::AsdcpWriterInfo>() };
            error::check(unsafe {
                asdcplib_sys::asdcp_as02_pcm_reader_fill_writer_info(self.ptr, &mut ffi)
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
                asdcplib_sys::asdcp_as02_pcm_reader_read_frame(
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
            unsafe { asdcplib_sys::asdcp_as02_pcm_reader_free(self.ptr) }
        }
    }
}

/// AS-02 Timed Text (subtitle) read/write.
pub mod timed_text {
    use crate::WriterInfo;
    use crate::crypto::{AesDecContext, AesEncContext, HmacContext};
    use crate::error::{self, Result};
    use crate::timed_text::TimedTextDescriptor;
    use std::ffi::CString;

    /// AS-02 Timed Text MXF writer.
    pub struct MxfWriter {
        ptr: *mut asdcplib_sys::AsdcpAs02TimedTextWriter,
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
                ptr: unsafe { asdcplib_sys::asdcp_as02_timed_text_writer_new() },
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
                asdcplib_sys::asdcp_as02_timed_text_writer_open_write(
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
                asdcplib_sys::asdcp_as02_timed_text_writer_write_timed_text_resource(
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
                asdcplib_sys::asdcp_as02_timed_text_writer_write_ancillary_resource(
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
            error::check(unsafe { asdcplib_sys::asdcp_as02_timed_text_writer_finalize(self.ptr) })
        }
    }

    impl Drop for MxfWriter {
        fn drop(&mut self) {
            unsafe { asdcplib_sys::asdcp_as02_timed_text_writer_free(self.ptr) }
        }
    }

    /// AS-02 Timed Text MXF reader.
    pub struct MxfReader {
        ptr: *mut asdcplib_sys::AsdcpAs02TimedTextReader,
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
                ptr: unsafe { asdcplib_sys::asdcp_as02_timed_text_reader_new() },
            }
        }

        pub fn open_read(&mut self, filename: &str) -> Result<()> {
            let cstr =
                CString::new(filename).map_err(|_| crate::Error::InvalidArgument("null byte"))?;
            error::check(unsafe {
                asdcplib_sys::asdcp_as02_timed_text_reader_open_read(self.ptr, cstr.as_ptr())
            })
        }

        pub fn close(&mut self) -> Result<()> {
            error::check(unsafe { asdcplib_sys::asdcp_as02_timed_text_reader_close(self.ptr) })
        }

        pub fn descriptor(&mut self) -> Result<TimedTextDescriptor> {
            let mut ffi = unsafe { std::mem::zeroed::<asdcplib_sys::AsdcpTimedTextDescriptor>() };
            error::check(unsafe {
                asdcplib_sys::asdcp_as02_timed_text_reader_fill_descriptor(self.ptr, &mut ffi)
            })?;
            Ok(TimedTextDescriptor::from_ffi(&ffi))
        }

        pub fn writer_info(&mut self) -> Result<WriterInfo> {
            let mut ffi = unsafe { std::mem::zeroed::<asdcplib_sys::AsdcpWriterInfo>() };
            error::check(unsafe {
                asdcplib_sys::asdcp_as02_timed_text_reader_fill_writer_info(self.ptr, &mut ffi)
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
                asdcplib_sys::asdcp_as02_timed_text_reader_read_timed_text_resource(
                    self.ptr,
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
            unsafe { asdcplib_sys::asdcp_as02_timed_text_reader_free(self.ptr) }
        }
    }
}

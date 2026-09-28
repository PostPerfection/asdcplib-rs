use crate::as02::pcm::SoundfieldGroupProperties;
use crate::error::{self, Result};
use crate::pcm::{mca_string, optional};
use crate::{Rational, WriterInfo};
use std::ffi::CString;

// ST 2067-201 frames each carry a preamble element then an IA frame element
const ELEMENT_TAG_BYTES: usize = 1;
const ELEMENT_LENGTH_BYTES: usize = 4;
const ELEMENT_HEADER_BYTES: usize = ELEMENT_TAG_BYTES + ELEMENT_LENGTH_BYTES;
const NOT_AN_IA_BITSTREAM_FRAME: &str = "not an IA bitstream frame: a preamble element then a non-empty IA frame element, each a tag byte, a big-endian u32 length and the value";

const INITIAL_FRAME_BUFFER_BYTES: usize = 64 * 1024;
const LARGEST_FRAME_BUFFER_BYTES: usize = u32::MAX as usize;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct IabEssenceDescriptor {
    pub instance_id: [u8; 16],
    pub generation_id: Option<[u8; 16]>,
    pub locators: Vec<[u8; 16]>,
    pub sub_descriptors: Vec<[u8; 16]>,
    pub linked_track_id: Option<u32>,
    pub sample_rate: Rational,
    pub container_duration: Option<u64>,
    pub essence_container: [u8; 16],
    pub codec: Option<[u8; 16]>,
    pub audio_sampling_rate: Rational,
    pub locked: bool,
    pub audio_ref_level: Option<u8>,
    pub electro_spatial_formulation: Option<u8>,
    pub channel_count: u32,
    pub quantization_bits: u32,
    pub dial_norm: Option<u8>,
    pub sound_essence_coding: [u8; 16],
    pub reference_audio_alignment_level: Option<u8>,
    pub reference_image_edit_rate: Option<Rational>,
}

impl IabEssenceDescriptor {
    fn from_ffi(ffi: &asdcplib_sys::AsdcpIabEssenceDescriptor) -> Self {
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
            audio_sampling_rate: Rational::from_ffi(&ffi.audio_sampling_rate),
            locked: ffi.locked != 0,
            audio_ref_level: optional(ffi.has_audio_ref_level, ffi.audio_ref_level),
            electro_spatial_formulation: optional(
                ffi.has_electro_spatial_formulation,
                ffi.electro_spatial_formulation,
            ),
            channel_count: ffi.channel_count,
            quantization_bits: ffi.quantization_bits,
            dial_norm: optional(ffi.has_dial_norm, ffi.dial_norm),
            sound_essence_coding: ffi.sound_essence_coding,
            reference_audio_alignment_level: optional(
                ffi.has_reference_audio_alignment_level,
                ffi.reference_audio_alignment_level,
            ),
            reference_image_edit_rate: optional(
                ffi.has_reference_image_edit_rate,
                Rational::from_ffi(&ffi.reference_image_edit_rate),
            ),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct IabSoundfieldLabel {
    pub instance_id: [u8; 16],
    pub tag_symbol: String,
    pub tag_name: Option<String>,
    pub label_dictionary_id: [u8; 16],
    pub link_id: [u8; 16],
    pub spoken_language: Option<String>,
    pub title: Option<String>,
    pub title_version: Option<String>,
    pub audio_content_kind: Option<String>,
    pub audio_element_kind: Option<String>,
}

impl IabSoundfieldLabel {
    fn from_ffi(ffi: &asdcplib_sys::AsdcpMcaLabel) -> Result<Self> {
        Ok(Self {
            instance_id: ffi.instance_id,
            tag_symbol: mca_string(&ffi.tag_symbol)?,
            tag_name: optional(ffi.has_tag_name, mca_string(&ffi.tag_name)?),
            label_dictionary_id: ffi.label_dictionary_id,
            link_id: ffi.link_id,
            spoken_language: optional(ffi.has_spoken_language, mca_string(&ffi.spoken_language)?),
            title: optional(ffi.has_title, mca_string(&ffi.title)?),
            title_version: optional(ffi.has_title_version, mca_string(&ffi.title_version)?),
            audio_content_kind: optional(
                ffi.has_audio_content_kind,
                mca_string(&ffi.audio_content_kind)?,
            ),
            audio_element_kind: optional(
                ffi.has_audio_element_kind,
                mca_string(&ffi.audio_element_kind)?,
            ),
        })
    }
}

fn element_value_length(frame: &[u8], element_offset: usize) -> Option<usize> {
    let length_offset = element_offset.checked_add(ELEMENT_TAG_BYTES)?;
    let length_bytes =
        frame.get(length_offset..length_offset.checked_add(ELEMENT_LENGTH_BYTES)?)?;
    Some(u32::from_be_bytes(length_bytes.try_into().ok()?) as usize)
}

// the reader parses these two lengths to find where a frame ends
fn frame_has_ia_bitstream_layout(frame: &[u8]) -> bool {
    let Some(preamble_length) = element_value_length(frame, 0) else {
        return false;
    };
    let ia_frame_offset = ELEMENT_HEADER_BYTES + preamble_length;
    let Some(ia_frame_length) = element_value_length(frame, ia_frame_offset) else {
        return false;
    };
    // the reader returns an empty frame for an empty IA frame element
    ia_frame_length > 0 && ia_frame_offset + ELEMENT_HEADER_BYTES + ia_frame_length == frame.len()
}

pub struct MxfWriter {
    ptr: *mut asdcplib_sys::AsdcpAs02IabWriter,
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
            ptr: unsafe { asdcplib_sys::asdcp_as02_iab_writer_new() },
        }
    }

    pub fn open_write(
        &mut self,
        filename: &str,
        info: &WriterInfo,
        soundfield: &SoundfieldGroupProperties,
        edit_rate: Rational,
        sampling_rate: Rational,
        reference_audio_alignment_level: i8,
    ) -> Result<()> {
        if info.encrypted_essence {
            return Err(crate::Error::InvalidArgument(
                "asdcplib writes IAB track files in the clear only",
            ));
        }
        let cstr = CString::new(filename)
            .map_err(|_| crate::Error::InvalidArgument("null byte in filename"))?;
        let properties = soundfield.to_cstrings()?;
        let ffi_properties = asdcplib_sys::AsdcpSoundfieldGroupProperties {
            language: properties[0].as_ptr(),
            title: properties[1].as_ptr(),
            title_version: properties[2].as_ptr(),
            audio_content_kind: properties[3].as_ptr(),
            audio_element_kind: properties[4].as_ptr(),
        };
        let ffi_info = info.to_ffi();
        error::check(unsafe {
            asdcplib_sys::asdcp_as02_iab_writer_open_write(
                self.ptr,
                cstr.as_ptr(),
                &ffi_info,
                &ffi_properties,
                edit_rate.to_ffi(),
                sampling_rate.to_ffi(),
                reference_audio_alignment_level,
            )
        })
    }

    pub fn write_frame(&mut self, frame: &[u8]) -> Result<()> {
        if !frame_has_ia_bitstream_layout(frame) {
            return Err(crate::Error::InvalidArgument(NOT_AN_IA_BITSTREAM_FRAME));
        }
        let frame_size = u32::try_from(frame.len())
            .map_err(|_| crate::Error::InvalidArgument("IA bitstream frame exceeds 4 GiB"))?;
        error::check(unsafe {
            asdcplib_sys::asdcp_as02_iab_writer_write_frame(self.ptr, frame.as_ptr(), frame_size)
        })
    }

    pub fn finalize(&mut self) -> Result<()> {
        error::check(unsafe { asdcplib_sys::asdcp_as02_iab_writer_finalize(self.ptr) })
    }
}

impl Drop for MxfWriter {
    fn drop(&mut self) {
        unsafe { asdcplib_sys::asdcp_as02_iab_writer_free(self.ptr) }
    }
}

pub struct MxfReader {
    ptr: *mut asdcplib_sys::AsdcpAs02IabReader,
    frame_buffer: Vec<u8>,
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
            ptr: unsafe { asdcplib_sys::asdcp_as02_iab_reader_new() },
            frame_buffer: vec![0; INITIAL_FRAME_BUFFER_BYTES],
        }
    }

    pub fn open_read(&mut self, filename: &str) -> Result<()> {
        let cstr = CString::new(filename)
            .map_err(|_| crate::Error::InvalidArgument("null byte in filename"))?;
        error::check(unsafe {
            asdcplib_sys::asdcp_as02_iab_reader_open_read(self.ptr, cstr.as_ptr())
        })
    }

    pub fn close(&mut self) -> Result<()> {
        error::check(unsafe { asdcplib_sys::asdcp_as02_iab_reader_close(self.ptr) })
    }

    pub fn writer_info(&mut self) -> Result<WriterInfo> {
        let mut ffi = unsafe { std::mem::zeroed::<asdcplib_sys::AsdcpWriterInfo>() };
        error::check(unsafe {
            asdcplib_sys::asdcp_as02_iab_reader_fill_writer_info(self.ptr, &mut ffi)
        })?;
        Ok(WriterInfo::from_ffi(&ffi))
    }

    pub fn frame_count(&mut self) -> Result<u32> {
        let mut count: u32 = 0;
        error::check(unsafe {
            asdcplib_sys::asdcp_as02_iab_reader_frame_count(self.ptr, &mut count)
        })?;
        Ok(count)
    }

    pub fn iab_essence_descriptor(&mut self) -> Result<IabEssenceDescriptor> {
        let mut ffi = unsafe { std::mem::zeroed::<asdcplib_sys::AsdcpIabEssenceDescriptor>() };
        error::check(unsafe {
            asdcplib_sys::asdcp_as02_iab_reader_read_iab_essence_descriptor(self.ptr, &mut ffi)
        })?;
        Ok(IabEssenceDescriptor::from_ffi(&ffi))
    }

    pub fn soundfield_label(&mut self) -> Result<IabSoundfieldLabel> {
        let mut ffi = unsafe { std::mem::zeroed::<asdcplib_sys::AsdcpMcaLabel>() };
        error::check(unsafe {
            asdcplib_sys::asdcp_as02_iab_reader_read_soundfield_label(self.ptr, &mut ffi)
        })?;
        IabSoundfieldLabel::from_ffi(&ffi)
    }

    pub fn read_frame(&mut self, frame_number: u32) -> Result<&[u8]> {
        loop {
            let mut out_size: u32 = 0;
            let result = unsafe {
                asdcplib_sys::asdcp_as02_iab_reader_read_frame(
                    self.ptr,
                    frame_number,
                    self.frame_buffer.as_mut_ptr(),
                    self.frame_buffer.len() as u32,
                    &mut out_size,
                )
            };
            // a short buffer comes back without the size the frame needs
            if result == asdcplib_sys::RESULT_SMALLBUF
                && self.frame_buffer.len() < LARGEST_FRAME_BUFFER_BYTES
            {
                let grown = self
                    .frame_buffer
                    .len()
                    .saturating_mul(2)
                    .min(LARGEST_FRAME_BUFFER_BYTES);
                self.frame_buffer = vec![0; grown];
                continue;
            }
            error::check(result)?;
            return Ok(&self.frame_buffer[..out_size as usize]);
        }
    }
}

impl Drop for MxfReader {
    fn drop(&mut self) {
        unsafe { asdcplib_sys::asdcp_as02_iab_reader_free(self.ptr) }
    }
}

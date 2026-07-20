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
    use crate::WriterInfo;
    use crate::crypto::{AesDecContext, AesEncContext, HmacContext};
    use crate::error::{self, Result};
    use crate::jp2k::PictureDescriptor;
    use std::ffi::CString;

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

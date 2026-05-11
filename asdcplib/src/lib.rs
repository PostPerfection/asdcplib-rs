//! Safe Rust wrapper for asdcplib — AS-DCP and AS-02 MXF file access.
//!
//! Provides safe abstractions over the raw FFI bindings in `asdcplib-sys`.
//!
//! # Supported essence types
//! - JPEG 2000 (mono and stereoscopic 3D)
//! - PCM audio (24-bit, 48kHz and 96kHz)
//! - Timed Text (SMPTE ST 429-5)
//! - Dolby Atmos (IAB)
//!
//! # Example
//! ```no_run
//! use asdcplib::{EssenceType, essence_type};
//!
//! let etype = essence_type("picture.mxf").unwrap();
//! assert_eq!(etype, EssenceType::Jpeg2000);
//! ```

pub mod atmos;
pub mod crypto;
mod error;
pub mod jp2k;
pub mod pcm;
pub mod timed_text;

pub use error::{Error, Result};

use std::ffi::CString;

/// MXF label set (determines SMPTE vs Interop labeling).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LabelSet {
    Unknown = 0,
    Interop = 1,
    Smpte = 2,
}

/// Essence type detected in an MXF file.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum EssenceType {
    Unknown,
    Mpeg2Ves,
    Jpeg2000,
    Pcm24b48k,
    Pcm24b96k,
    TimedText,
    Jpeg2000Stereo,
    DcDataUnknown,
    DcDataDolbyAtmos,
}

impl From<i32> for EssenceType {
    fn from(v: i32) -> Self {
        match v {
            1 => EssenceType::Mpeg2Ves,
            2 => EssenceType::Jpeg2000,
            3 => EssenceType::Pcm24b48k,
            4 => EssenceType::Pcm24b96k,
            5 => EssenceType::TimedText,
            6 => EssenceType::Jpeg2000Stereo,
            7 => EssenceType::DcDataUnknown,
            8 => EssenceType::DcDataDolbyAtmos,
            _ => EssenceType::Unknown,
        }
    }
}

/// Writer identification information for MXF files.
#[derive(Debug, Clone)]
pub struct WriterInfo {
    pub product_uuid: [u8; 16],
    pub asset_uuid: [u8; 16],
    pub context_id: [u8; 16],
    pub cryptographic_key_id: [u8; 16],
    pub encrypted_essence: bool,
    pub uses_hmac: bool,
    pub label_set: LabelSet,
}

impl Default for WriterInfo {
    fn default() -> Self {
        Self {
            product_uuid: [0; 16],
            asset_uuid: [0; 16],
            context_id: [0; 16],
            cryptographic_key_id: [0; 16],
            encrypted_essence: false,
            uses_hmac: false,
            label_set: LabelSet::Smpte,
        }
    }
}

impl WriterInfo {
    pub(crate) fn to_ffi(&self) -> asdcplib_sys::AsdcpWriterInfo {
        asdcplib_sys::AsdcpWriterInfo {
            product_uuid: self.product_uuid,
            asset_uuid: self.asset_uuid,
            context_id: self.context_id,
            cryptographic_key_id: self.cryptographic_key_id,
            encrypted_essence: if self.encrypted_essence { 1 } else { 0 },
            uses_hmac: if self.uses_hmac { 1 } else { 0 },
            label_set_type: self.label_set as i32,
        }
    }

    pub(crate) fn from_ffi(ffi: &asdcplib_sys::AsdcpWriterInfo) -> Self {
        Self {
            product_uuid: ffi.product_uuid,
            asset_uuid: ffi.asset_uuid,
            context_id: ffi.context_id,
            cryptographic_key_id: ffi.cryptographic_key_id,
            encrypted_essence: ffi.encrypted_essence != 0,
            uses_hmac: ffi.uses_hmac != 0,
            label_set: match ffi.label_set_type {
                1 => LabelSet::Interop,
                2 => LabelSet::Smpte,
                _ => LabelSet::Unknown,
            },
        }
    }
}

/// Rational number (e.g., frame rate, sample rate).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Rational {
    pub numerator: i32,
    pub denominator: i32,
}

impl Rational {
    pub fn new(n: i32, d: i32) -> Self {
        Self {
            numerator: n,
            denominator: d,
        }
    }

    pub fn quotient(&self) -> f64 {
        self.numerator as f64 / self.denominator as f64
    }

    pub(crate) fn to_ffi(self) -> asdcplib_sys::AsdcpRational {
        asdcplib_sys::AsdcpRational {
            numerator: self.numerator,
            denominator: self.denominator,
        }
    }

    pub(crate) fn from_ffi(ffi: &asdcplib_sys::AsdcpRational) -> Self {
        Self {
            numerator: ffi.numerator,
            denominator: ffi.denominator,
        }
    }
}

// Common edit rates
pub const EDIT_RATE_24: Rational = Rational {
    numerator: 24,
    denominator: 1,
};
pub const EDIT_RATE_25: Rational = Rational {
    numerator: 25,
    denominator: 1,
};
pub const EDIT_RATE_30: Rational = Rational {
    numerator: 30,
    denominator: 1,
};
pub const EDIT_RATE_48: Rational = Rational {
    numerator: 48,
    denominator: 1,
};
pub const EDIT_RATE_60: Rational = Rational {
    numerator: 60,
    denominator: 1,
};
pub const SAMPLE_RATE_48K: Rational = Rational {
    numerator: 48000,
    denominator: 1,
};
pub const SAMPLE_RATE_96K: Rational = Rational {
    numerator: 96000,
    denominator: 1,
};

/// Return the asdcplib version string.
pub fn version() -> String {
    unsafe {
        let ptr = asdcplib_sys::asdcp_version();
        std::ffi::CStr::from_ptr(ptr).to_string_lossy().into_owned()
    }
}

/// Detect the essence type of an MXF file.
pub fn essence_type(filename: &str) -> Result<EssenceType> {
    let cstr =
        CString::new(filename).map_err(|_| Error::InvalidArgument("null byte in filename"))?;
    let mut etype: i32 = 0;
    let result = unsafe { asdcplib_sys::asdcp_essence_type(cstr.as_ptr(), &mut etype) };
    error::check(result)?;
    Ok(EssenceType::from(etype))
}

/// Detect the essence type of a raw (unwrapped) file.
pub fn raw_essence_type(filename: &str) -> Result<EssenceType> {
    let cstr =
        CString::new(filename).map_err(|_| Error::InvalidArgument("null byte in filename"))?;
    let mut etype: i32 = 0;
    let result = unsafe { asdcplib_sys::asdcp_raw_essence_type(cstr.as_ptr(), &mut etype) };
    error::check(result)?;
    Ok(EssenceType::from(etype))
}

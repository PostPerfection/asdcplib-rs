//! Cryptographic context wrappers (AES encryption/decryption, HMAC).

use crate::LabelSet;
use crate::error::{self, Result};

/// AES encryption context.
pub struct AesEncContext {
    ptr: *mut asdcplib_sys::AsdcpAesEncContext,
}

unsafe impl Send for AesEncContext {}

impl Default for AesEncContext {
    fn default() -> Self {
        Self::new()
    }
}

impl AesEncContext {
    pub fn new() -> Self {
        Self {
            ptr: unsafe { asdcplib_sys::asdcp_aes_enc_context_new() },
        }
    }

    pub fn init_key(&mut self, key: &[u8; 16]) -> Result<()> {
        error::check(unsafe {
            asdcplib_sys::asdcp_aes_enc_context_init_key(self.ptr, key.as_ptr())
        })
    }

    pub fn set_ivec(&mut self, ivec: &[u8; 16]) -> Result<()> {
        error::check(unsafe {
            asdcplib_sys::asdcp_aes_enc_context_set_ivec(self.ptr, ivec.as_ptr())
        })
    }

    pub(crate) fn as_mut_ptr(&mut self) -> *mut asdcplib_sys::AsdcpAesEncContext {
        self.ptr
    }
}

impl Drop for AesEncContext {
    fn drop(&mut self) {
        unsafe { asdcplib_sys::asdcp_aes_enc_context_free(self.ptr) }
    }
}

/// AES decryption context.
pub struct AesDecContext {
    ptr: *mut asdcplib_sys::AsdcpAesDecContext,
}

unsafe impl Send for AesDecContext {}

impl Default for AesDecContext {
    fn default() -> Self {
        Self::new()
    }
}

impl AesDecContext {
    pub fn new() -> Self {
        Self {
            ptr: unsafe { asdcplib_sys::asdcp_aes_dec_context_new() },
        }
    }

    pub fn init_key(&mut self, key: &[u8; 16]) -> Result<()> {
        error::check(unsafe {
            asdcplib_sys::asdcp_aes_dec_context_init_key(self.ptr, key.as_ptr())
        })
    }

    pub fn set_ivec(&mut self, ivec: &[u8; 16]) -> Result<()> {
        error::check(unsafe {
            asdcplib_sys::asdcp_aes_dec_context_set_ivec(self.ptr, ivec.as_ptr())
        })
    }

    pub(crate) fn as_mut_ptr(&mut self) -> *mut asdcplib_sys::AsdcpAesDecContext {
        self.ptr
    }
}

impl Drop for AesDecContext {
    fn drop(&mut self) {
        unsafe { asdcplib_sys::asdcp_aes_dec_context_free(self.ptr) }
    }
}

/// HMAC context for integrity verification.
pub struct HmacContext {
    ptr: *mut asdcplib_sys::AsdcpHmacContext,
}

unsafe impl Send for HmacContext {}

impl Default for HmacContext {
    fn default() -> Self {
        Self::new()
    }
}

impl HmacContext {
    pub fn new() -> Self {
        Self {
            ptr: unsafe { asdcplib_sys::asdcp_hmac_context_new() },
        }
    }

    pub fn init_key(&mut self, key: &[u8; 16], label_set: LabelSet) -> Result<()> {
        error::check(unsafe {
            asdcplib_sys::asdcp_hmac_context_init_key(self.ptr, key.as_ptr(), label_set as i32)
        })
    }

    pub(crate) fn as_mut_ptr(&mut self) -> *mut asdcplib_sys::AsdcpHmacContext {
        self.ptr
    }
}

impl Drop for HmacContext {
    fn drop(&mut self) {
        unsafe { asdcplib_sys::asdcp_hmac_context_free(self.ptr) }
    }
}

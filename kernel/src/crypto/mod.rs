// SPDX-License-Identifier: MIT OR Apache-2.0
//
// Copyright (C) 2023 IBM
//
// Author: Claudio Carvalho <cclaudio@linux.ibm.com>

//! SVSM kernel crypto API

extern crate alloc;

use alloc::boxed::Box;
use alloc::collections::TryReserveError;
use cocoon_tpm_crypto::hash::HashInstance;
use cocoon_tpm_tpm2_interface::{TpmiAlgHash, TpmtHa};
use cocoon_tpm_utils_common::io_slices::{IoSlicesIterCommon as _, SingletonIoSlice};
use zeroize::{Zeroize, ZeroizeOnDrop};

use crate::error::SvsmError;
use crate::utils::vec::vec_sized;

/// Container for sensitive byte data.
///
/// The owned buffer is zeroed when the `SecretSlice` is dropped or explicitly
/// [`zeroize`](Zeroize::zeroize)d.
#[derive(Zeroize, ZeroizeOnDrop)]
pub struct SecretSlice(Box<[u8]>);

impl SecretSlice {
    pub fn new_sized(size: usize) -> Result<Self, TryReserveError> {
        let v = vec_sized(size)?;
        Ok(Self(v.into_boxed_slice()))
    }
}

impl From<Box<[u8]>> for SecretSlice {
    fn from(data: Box<[u8]>) -> Self {
        Self(data)
    }
}

impl core::ops::Deref for SecretSlice {
    type Target = [u8];

    fn deref(&self) -> &[u8] {
        &self.0
    }
}

impl core::ops::DerefMut for SecretSlice {
    fn deref_mut(&mut self) -> &mut [u8] {
        &mut self.0
    }
}

impl core::fmt::Debug for SecretSlice {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        f.debug_struct("SecretSlice")
            .field("len", &self.0.len())
            .field("data", &"[REDACTED]")
            .finish()
    }
}

pub mod concat_kdf;

/// Digest size of SHA-512 in bytes.
const SHA512_DIGEST_SIZE: usize = 64;

/// One-shot SHA-512 digest over `input`.
///
/// Convenience wrapper over the streaming
/// [`HashInstance`](cocoon_tpm_crypto::hash::HashInstance): callers hashing
/// data assembled from multiple sources should feed it through separate
/// [`update`](cocoon_tpm_crypto::hash::HashInstance::update) calls instead
/// of concatenating it first.
pub fn sha512_digest(input: &[u8]) -> Result<[u8; SHA512_DIGEST_SIZE], SvsmError> {
    let mut hash = HashInstance::new(TpmiAlgHash::Sha512).map_err(|_| SvsmError::NotSupported)?;

    hash.update(SingletonIoSlice::new(input).map_infallible_err())
        .map_err(|_| SvsmError::NotSupported)?;

    match hash.finalize() {
        Ok(TpmtHa::Sha512(digest)) => (&*digest).try_into().map_err(|_| SvsmError::InvalidFormat),
        _ => Err(SvsmError::NotSupported),
    }
}

mod rng;
pub use rng::{SvsmRng, get_svsm_rng};

#[cfg(test)]
mod tests {
    use super::*;
    use alloc::vec::Vec;
    use zeroize::Zeroize;

    #[test]
    fn secretslice_stores_secret() {
        let b: SecretSlice = Vec::from([1u8, 2, 3, 4, 5]).into_boxed_slice().into();
        assert_eq!(b.len(), 5);
        assert_eq!(&*b, &[1, 2, 3, 4, 5]);
    }

    #[test]
    fn secretslice_empty_ok() {
        let b: SecretSlice = Vec::new().into_boxed_slice().into();
        assert_eq!(b.len(), 0);
        assert!(b.is_empty());
    }

    #[test]
    fn secretslice_deref_mut_mutates() {
        let mut b: SecretSlice = Vec::from([1u8, 2, 3, 4]).into_boxed_slice().into();
        b[0] = 42;
        assert_eq!(&*b, &[42, 2, 3, 4]);
    }

    #[test]
    fn secretslice_zeroize_wipes_data() {
        let mut b: SecretSlice = Vec::from([1u8, 2, 3, 4, 5, 6, 7, 8])
            .into_boxed_slice()
            .into();
        b.zeroize();
        assert!(b.iter().all(|&byte| byte == 0));
    }
}

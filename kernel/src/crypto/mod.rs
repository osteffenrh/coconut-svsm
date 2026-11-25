// SPDX-License-Identifier: MIT OR Apache-2.0
//
// Copyright (C) 2023 IBM
//
// Author: Claudio Carvalho <cclaudio@linux.ibm.com>

//! SVSM kernel crypto API

extern crate alloc;

use alloc::boxed::Box;
use alloc::collections::TryReserveError;
use zeroize::{Zeroize, ZeroizeOnDrop};

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

pub mod digest {
    //! API for message digests.

    extern crate alloc;

    use alloc::vec::Vec;

    // Note the typical new/update/finish API is not initially supported due
    // to our inability to hide an abstract implementation's state representation
    // behind a type without any fields.

    pub trait Algorithm {
        /// Digests `input` into an output vector of size `OUTPUT_LEN`.
        fn digest(input: &[u8]) -> Vec<u8>;
    }

    /// Sha512 type
    #[derive(Copy, Clone, Debug)]
    pub struct Sha512;
}

// Crypto implementations supported. Only one of them must be compiled-in.

pub mod rustcrypto;

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

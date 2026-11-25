// SPDX-License-Identifier: MIT OR Apache-2.0
//
// Copyright (C) 2023 IBM
//
// Author: Claudio Carvalho <cclaudio@linux.ibm.com>

//! Crypto adapters on top of cocoon-tpm-crypto.

extern crate alloc;

use alloc::vec::Vec;
use cocoon_tpm_crypto::CryptoError;
use cocoon_tpm_utils_common::io_slices::{IoSlicesIterCommon as _, SingletonIoSlice};

use crate::crypto::digest::{Algorithm as CryptoHashTrait, Sha512 as CryptoSha512};

impl CryptoHashTrait for CryptoSha512 {
    fn digest(input: &[u8]) -> Vec<u8> {
        let mut h = cocoon_tpm_crypto::hash::HashInstance::new(
            cocoon_tpm_tpm2_interface::TpmiAlgHash::Sha512,
        )
        .unwrap();
        let iter = SingletonIoSlice::new(input).map_infallible_err::<CryptoError>();
        h.update(iter).unwrap();
        let res = h.finalize().unwrap();
        let cocoon_tpm_tpm2_interface::TpmtHa::Sha512(buf) = res else {
            unreachable!();
        };
        buf.to_vec()
    }
}

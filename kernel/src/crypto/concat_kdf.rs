use cocoon_tpm_crypto::CryptoError;
use cocoon_tpm_tpm2_interface::TpmiAlgHash;
use cocoon_tpm_utils_common::io_slices::{IoSlicesIterCommon, SingletonIoSlice};

/// Concat KDF (NIST SP 800-56A, section 5.8.1) using SHA-256.
///
/// Derives `key` from `secret` and `other_info` via:
///   for each hash-sized chunk of output:
///       hash(counter_be32 || secret || other_info), counter starts at 1
///
/// Based on <https://docs.rs/concat-kdf/0.1.0/src/concat_kdf/lib.rs.html#58-90>
pub fn concat_kdf_sha256(
    secret: &[u8],
    other_info: &[u8],
    key: &mut [u8],
) -> Result<(), CryptoError> {
    const SHA256_OUTPUT_SIZE: usize = 32;

    if secret.is_empty() || key.is_empty() {
        return Err(CryptoError::Internal);
    }

    // Check for counter overflow
    if size_of::<usize>() > 4 && key.len() >= SHA256_OUTPUT_SIZE * (u32::MAX as usize) {
        return Err(CryptoError::Internal);
    }

    let mut counter: u32 = 1;
    for chunk in key.chunks_mut(SHA256_OUTPUT_SIZE) {
        let mut h = cocoon_tpm_crypto::hash::HashInstance::new(TpmiAlgHash::Sha256)
            .expect("SHA-256 must be supported");
        h.update(
            SingletonIoSlice::new(&counter.to_be_bytes()).map_infallible_err::<CryptoError>(),
        )?;
        h.update(SingletonIoSlice::new(secret).map_infallible_err::<CryptoError>())?;
        h.update(SingletonIoSlice::new(other_info).map_infallible_err::<CryptoError>())?;
        let res = h.finalize()?;
        let cocoon_tpm_tpm2_interface::TpmtHa::Sha256(buf) = res else {
            unreachable!();
        };
        chunk.copy_from_slice(&buf[..chunk.len()]);
        counter += 1;
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn empty_secret_is_rejected() {
        let mut key = [0u8; 32];
        assert!(concat_kdf_sha256(&[], &[], &mut key).is_err());
    }

    #[test]
    fn empty_key_is_rejected() {
        assert!(concat_kdf_sha256(&[0u8; 16], &[], &mut []).is_err());
    }

    #[test]
    fn single_block_no_other_info() {
        // SHA-256(00000001 || 00)
        let mut key = [0u8; 32];
        concat_kdf_sha256(&[0x00], &[], &mut key).unwrap();
        assert_eq!(
            key,
            [
                0x06, 0x0d, 0xc6, 0x3e, 0x55, 0x95, 0xdf, 0xfb, 0xd1, 0x61, 0xc9, 0xec, 0x98, 0xbc,
                0x06, 0xfc, 0xf6, 0x7c, 0xb2, 0x2e, 0x2e, 0x75, 0xec, 0xdf, 0x00, 0x03, 0x82, 0x13,
                0x88, 0xae, 0xee, 0x4d,
            ]
        );
    }

    #[test]
    fn single_block_with_other_info() {
        // SHA-256(00000001 || 00 || 00)
        let mut key = [0u8; 32];
        concat_kdf_sha256(&[0x00], &[0x00], &mut key).unwrap();
        assert_eq!(
            key,
            [
                0x10, 0x48, 0x7d, 0x86, 0xce, 0x35, 0x84, 0xe1, 0x56, 0x87, 0x4e, 0xd9, 0xb2, 0x65,
                0x0e, 0x8d, 0x77, 0x2a, 0x8d, 0x1f, 0xdb, 0xb1, 0xc9, 0x11, 0x1b, 0xf7, 0xe2, 0xfb,
                0xca, 0xb1, 0x8c, 0xcb,
            ]
        );
    }

    #[test]
    fn multi_block_derivation() {
        let secret = &[
            0xba, 0x5e, 0xba, 0x11, 0xbe, 0xda, 0xbb, 0x1e, 0xbe, 0x50, 0x77, 0xed, 0xb0, 0xa7,
            0x10, 0xad, 0xb0, 0x1d, 0xfa, 0xce, 0xca, 0xb0, 0x05, 0xec, 0xa1, 0x1a, 0xb1, 0xec,
            0xa5, 0x5e, 0x77, 0xe0, 0x11,
        ];
        let other_info = &[0xf0, 0x05, 0xba, 0x11, 0x00, 0xdd, 0xba, 0x11];
        let expected: &[u8] = &[
            0xa4, 0x00, 0xbe, 0x99, 0x35, 0xd0, 0xc8, 0x43, 0xa1, 0x50, 0x4a, 0xa6, 0x4f, 0x60,
            0x78, 0x19, 0x5b, 0x6e, 0x20, 0xea, 0x3f, 0xe6, 0x4b, 0xb8, 0xd7, 0xf2, 0x9a, 0xae,
            0xa6, 0xa3, 0x51, 0xbf, 0x7e, 0x40, 0xca, 0xfe, 0x54, 0xe8, 0x6c, 0x4d, 0x50, 0x2f,
            0x82, 0xa3, 0x90, 0xab, 0x77, 0x09, 0x8e, 0x8c, 0xcc, 0xf9, 0x05, 0xb5, 0x82, 0x6d,
            0x47, 0x5e, 0x83, 0x16, 0x58, 0x3e, 0xb5, 0x3f,
        ];

        let mut key = [0u8; 64];
        concat_kdf_sha256(secret, other_info, &mut key).unwrap();
        assert_eq!(&key[..], expected);
    }

    #[test]
    fn partial_block_derivation() {
        // Request fewer bytes than a full hash block — the function
        // must still produce the correct prefix.
        let mut key_full = [0u8; 32];
        concat_kdf_sha256(&[0x00], &[], &mut key_full).unwrap();

        let mut key_short = [0u8; 16];
        concat_kdf_sha256(&[0x00], &[], &mut key_short).unwrap();
        assert_eq!(&key_short, &key_full[..16]);
    }
}

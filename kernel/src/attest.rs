// SPDX-License-Identifier: MIT OR Apache-2.0
//
// Copyright (c) 2024 Red Hat, Inc
//
// Author: Stefano Garzarella <sgarzare@redhat.com>
// Author: Tyler Fanelli <tfanelli@redhat.com>

extern crate alloc;

use crate::crypto::concat_kdf;
use crate::{
    boot_params::BootParams,
    crypto::{SecretSlice, get_svsm_rng},
    error::SvsmError,
    greq::{pld_report::*, services::get_regular_report},
    io::{Read, Write},
    utils::vec::{try_to_vec, vec_sized},
};
#[cfg(feature = "attest-serial")]
use crate::{io::DEFAULT_IO_DRIVER, serial::SerialPort};
use crate::{vsock::VMADDR_CID_HOST, vsock::VMADDR_PORT_ANY, vsock::stream::VsockStream};
use alloc::{string::ToString, vec::Vec};
use cocoon_tpm_crypto::{
    CryptoError,
    aead::{AeadAlg, AeadInstance},
    ecc::{EccKey, curve::Curve, ecdh::ecdh_c_1_1_cdh_compute_z},
    keywrap,
};
use cocoon_tpm_tpm2_interface::TpmiAlgHash;
use cocoon_tpm_tpm2_interface::{TpmEccCurve, TpmsEccPoint};
use cocoon_tpm_utils_common::io_slices::{
    IoSlicesIterCommon as _, SingletonIoSlice, SingletonIoSliceMut,
};
use kbs_types::Tee;
use libaproxy::*;
use serde::Serialize;
use zerocopy::{FromBytes, IntoBytes};

#[cfg(feature = "attest-serial")]
// TODO: Make the IO port configurable/discoverable or drop the support entirely.
const ATTEST_DEFAULT_SERIAL_IO_ADDR: u16 = 0x3e8; // COM3

enum Transport {
    Vsock(VsockStream),
    #[cfg(feature = "attest-serial")]
    Serial(SerialPort<'static>),
}

impl Read for Transport {
    type Err = SvsmError;

    fn read(&mut self, buf: &mut [u8]) -> Result<usize, Self::Err> {
        match self {
            Transport::Vsock(vsock) => vsock.read(buf),
            #[cfg(feature = "attest-serial")]
            Transport::Serial(serial) => serial.read(buf),
        }
    }
}

impl Write for Transport {
    type Err = SvsmError;

    fn write(&mut self, buf: &[u8]) -> Result<usize, Self::Err> {
        match self {
            Transport::Vsock(vsock) => vsock.write(buf),
            #[cfg(feature = "attest-serial")]
            Transport::Serial(serial) => serial.write(buf),
        }
    }
}

impl Transport {
    fn new(boot_params: &BootParams<'_>) -> Result<Self, SvsmError> {
        let vsock_port = match boot_params.attestation_vsock_port() {
            Some(0) | Some(VMADDR_PORT_ANY) | None => ATTEST_DEFAULT_VSOCK_PORT,
            Some(port) => port,
        };

        match VsockStream::connect(vsock_port, VMADDR_CID_HOST) {
            Ok(value) => Ok(Transport::Vsock(value)),
            Err(e) => {
                log::warn!(
                    "Failed to connect to attestation proxy on vsock port \
                     {vsock_port}: {e:?}."
                );

                #[cfg(feature = "attest-serial")]
                {
                    log::warn!("Falling back to serial port transport.");
                    create_serial_transport()
                }
                #[cfg(not(feature = "attest-serial"))]
                Err(e)
            }
        }
    }
}

#[cfg(feature = "attest-serial")]
fn create_serial_transport() -> Result<Transport, SvsmError> {
    let sp = SerialPort::new(&DEFAULT_IO_DRIVER, ATTEST_DEFAULT_SERIAL_IO_ADDR);
    sp.init();
    Ok(Transport::Serial(sp))
}

/// The attestation driver that communicates with the proxy via some communication channel (serial
/// port, virtio-vsock, etc...).
#[allow(missing_debug_implementations)]
pub struct AttestationDriver {
    transport: Transport,
    tee: Tee,
    ecc: EccKey,
}

impl AttestationDriver {
    /// Create an attestation driver connected to the attestation proxy.
    pub fn new(tee: Tee, boot_params: &BootParams<'_>) -> Result<Self, SvsmError> {
        match tee {
            Tee::Snp => (),
            _ => return Err(AttestationError::UnsupportedTee.into()),
        }

        let curve = Curve::new(TpmEccCurve::NistP521).map_err(AttestationError::Crypto)?;
        let ecc = sc_key_generate(&curve).map_err(AttestationError::Crypto)?;

        let transport = Transport::new(boot_params)?;
        Ok(Self {
            transport,
            tee,
            ecc,
        })
    }

    /// Attest SVSM's launch state by communicating with the attestation proxy.
    pub fn attest(&mut self) -> Result<SecretSlice, SvsmError> {
        let negotiation = self.negotiation()?;

        Ok(self.attestation(negotiation)?)
    }

    /// Send a negotiation request to the proxy. Proxy should reply with Negotiation parameters
    /// that should be included in attestation evidence (e.g. through SEV-SNP's REPORT_DATA
    /// mechanism).
    fn negotiation(&mut self) -> Result<NegotiationResponse, AttestationError> {
        let request = NegotiationRequest {
            version: (0, 1, 0), // Only version supported at present.
            tee: self.tee,
        };

        self.write(request)?;
        let payload = self.read()?;

        serde_json::from_slice(&payload).or(Err(AttestationError::NegotiationDeserialize))
    }

    /// Send an attestation request to the proxy. Proxy should reply with attestation response
    /// containing the status (success/fail) and an optional secret returned from the server upon
    /// successful attestation.
    fn attestation(&mut self, n: NegotiationResponse) -> Result<SecretSlice, AttestationError> {
        let curve =
            Curve::new(self.ecc.pub_key().get_curve_id()).map_err(AttestationError::Crypto)?;

        let pub_key = self
            .ecc
            .pub_key()
            .to_tpms_ecc_point(&curve.curve_ops().map_err(AttestationError::Crypto)?)
            .map_err(AttestationError::Crypto)?;

        let evidence = evidence(&self.tee, hash(&n, &pub_key)?)?;

        let req = AttestationRequest {
            tee: self.tee,
            evidence,
            challenge: n.challenge.clone(),
            key: (self.ecc.pub_key().get_curve_id(), &pub_key).into(),
        };

        self.write(req)?;
        let payload = self.read()?;

        let response: AttestationResponse = serde_json::from_slice(&payload)
            .map_err(|_| AttestationError::AttestationDeserialize)?;

        if !response.success {
            return Err(AttestationError::Failed);
        }

        let Some(decryption) = response.decryption else {
            return Err(AttestationError::PublicKeyMissing)?;
        };

        let Some(secret_enc) = response.secret else {
            return Err(AttestationError::SecretMissing);
        };

        // `secret_enc` holds ciphertext from the attestation server. Move it
        // into `SecretSlice` before decryption so the same buffer is decrypted
        // in-place and zeroed on drop.
        let mut secret_slice = SecretSlice::from(secret_enc.into_boxed_slice());

        self.decrypt(&mut secret_slice, decryption)?;

        Ok(secret_slice)
    }

    /// Decrypt a secret from the attestation server with the TEE private key. Secrets are
    /// encrypted with ECDH-ES+A256KW as described in RFC 7518, section 4.6.2.
    fn decrypt(&self, secret: &mut [u8], decryption: AesGcmData) -> Result<(), AttestationError> {
        let epk: TpmsEccPoint<'static> = decryption.epk.into();
        let z = ecdh_c_1_1_cdh_compute_z(&self.ecc, &epk).map_err(AttestationError::Crypto)?;

        let mut kdm = Vec::new();
        let alg_str = "ECDH-ES+A256KW".to_string();

        kdm.extend_from_slice(&(alg_str.len() as u32).to_be_bytes());
        kdm.extend_from_slice(alg_str.as_bytes());
        kdm.extend_from_slice(&(0_u32).to_be_bytes());
        kdm.extend_from_slice(&(0_u32).to_be_bytes());
        kdm.extend_from_slice(&(256_u32).to_be_bytes());

        let mut wrapping_key = SecretSlice::new_sized(32).or(Err(AttestationError::VecAlloc))?;

        concat_kdf::concat_kdf_sha256(&z, &kdm, &mut wrapping_key)
            .map_err(AttestationError::KeyDerivation)?;

        // A256KW is the unpadded AES key wrap of RFC 7518, section 4.4
        // (RFC 3394): the unwrapped CEK is 8 bytes shorter than the wrapped
        // CEK. All response data is untrusted, so reject malformed lengths
        // instead of relying on the subtraction to not underflow.
        let cek_len = decryption
            .wrapped_cek
            .len()
            .checked_sub(8)
            .ok_or(AttestationError::CekUnwrap)?;
        let mut cek = SecretSlice::new_sized(cek_len).or(Err(AttestationError::VecAlloc))?;

        keywrap::aes_kw_unwrap(&wrapping_key, &decryption.wrapped_cek, &mut cek)
            .map_err(|_| AttestationError::CekUnwrap)?;

        // Decrypt the secret in place, with the IV, AAD and detached
        // authentication tag taken from the response.
        let mut aead =
            AeadInstance::new(AeadAlg::Aes256Gcm, &cek).map_err(AttestationError::Crypto)?;

        aead.decrypt_in_place(
            &decryption.iv,
            SingletonIoSlice::new(&decryption.aad).map_infallible_err(),
            SingletonIoSliceMut::new(secret).map_infallible_err(),
            &decryption.tag,
        )
        .map_err(AttestationError::SecretDecrypt)?;

        Ok(())
    }

    /// Read attestation data from the transport channel.
    fn read(&mut self) -> Result<Vec<u8>, AttestationError> {
        let len = {
            let mut bytes = [0u8; 8];
            self.transport
                .read_exact(&mut bytes)
                .or(Err(AttestationError::ProxyRead))?;

            usize::from_ne_bytes(bytes)
        };

        let mut buf: Vec<u8> = vec_sized(len).or(Err(AttestationError::VecAlloc))?;

        self.transport
            .read_exact(&mut buf)
            .or(Err(AttestationError::ProxyRead))?;

        Ok(buf)
    }

    /// Write attestation data over the transport channel.
    fn write(&mut self, param: impl Serialize) -> Result<(), AttestationError> {
        let bytes = serde_json::to_vec(&param).or(Err(AttestationError::NegotiationSerialize))?;

        // The receiving party is unaware of how many bytes to read from the port. Write an 8-byte
        // header indicating the length of the buffer before writing the buffer itself.
        self.transport
            .write_all(&bytes.len().to_ne_bytes())
            .or(Err(AttestationError::ProxyWrite))?;
        self.transport
            .write_all(&bytes)
            .or(Err(AttestationError::ProxyWrite))?;

        Ok(())
    }
}

/// Possible errors when attesting TEE evidence.
#[derive(Clone, Copy, Debug)]
pub enum AttestationError {
    /// Error generating AES key.
    AesGenerate,
    /// Error deserializing the attestation response from JSON bytes.
    AttestationDeserialize,
    // Unable to unwrap Content Encryption Key (CEK).
    CekUnwrap,
    /// Unable to generate secure channel key.
    Crypto(CryptoError),
    /// Guest has failed attestation.
    Failed,
    // Unable to derive wrap key.
    KeyDerivation(CryptoError),
    /// Error deserializing the negotiation response from JSON bytes.
    NegotiationDeserialize,
    /// Error serializing the negotiation request to JSON bytes.
    NegotiationSerialize,
    /// Error reading from the attestation proxy transport channel.
    ProxyRead,
    /// Error writing over the attestation proxy transport channel.
    ProxyWrite,
    /// Attestation successful, but no public key found.
    PublicKeyMissing,
    /// Attestation successful, but unable to decrypt secret.
    SecretDecrypt(CryptoError),
    /// Attestation successful, but no secret found.
    SecretMissing,
    /// Unable to fetch SEV-SNP attestation report.
    SnpGetReport,
    /// Unsupported TEE architecture.
    UnsupportedTee,
    /// Unable to allocate memory for Vec.
    VecAlloc,
}

impl From<AttestationError> for SvsmError {
    fn from(e: AttestationError) -> Self {
        Self::TeeAttestation(e)
    }
}

/// Generate a key used to establish a secure channel between the confidential guest and
/// attestation server.
fn sc_key_generate(curve: &Curve) -> Result<EccKey, CryptoError> {
    let mut rng = get_svsm_rng()?;
    let curve_ops = curve.curve_ops()?;
    EccKey::generate(&curve_ops, &mut rng, None)
}

/// Hash negotiation parameters and fetch TEE evidence.
fn evidence(tee: &Tee, hash: Vec<u8>) -> Result<AttestationEvidence, AttestationError> {
    let evidence = match tee {
        &Tee::Snp => {
            let mut user_data = [0u8; 64];
            user_data.copy_from_slice(&hash);

            let request = SnpReportRequest::new(user_data, 0, 1);

            let data = try_to_vec(request.as_bytes()).or(Err(AttestationError::VecAlloc))?;
            // The buffer currently contains the the SnpReportRequest structure. However, SVSM
            // will fill this buffer in with the SnpReportResponse when fetching the report.
            // Ensure the array is large enough to contain the response (which is much larger
            // than the request, as it contains the attestation report).
            let mut buf: Vec<u8> = vec_sized(2048).or(Err(AttestationError::VecAlloc))?;

            buf[..data.len()].copy_from_slice(&data);

            let len = get_regular_report(&mut buf).or(Err(AttestationError::SnpGetReport))?;

            // We have the length of the response. The rest of the response is unused.
            // Parse the SnpReportResponse from the slice of the buf containing the
            // response (that is, &buf[0..len]).
            let resp = SnpReportResponse::ref_from_bytes(&buf[..len])
                .or(Err(AttestationError::SnpGetReport))?;

            // Get the attestation report as bytes for serialization in the
            // AttestationRequest.
            let report =
                try_to_vec(resp.report().as_bytes()).or(Err(AttestationError::VecAlloc))?;

            AttestationEvidence::Snp {
                report,
                certs_buf: None,
            }
        }
        // We check for supported TEE architectures in the AttestationDriver's constructor.
        _ => unreachable!(),
    };

    Ok(evidence)
}

/// Hash the negotiation parameters from the attestation server for inclusion in the
/// attestation evidence.
fn hash(
    n: &NegotiationResponse,
    pub_key: &TpmsEccPoint<'static>,
) -> Result<Vec<u8>, AttestationError> {
    let mut sha = cocoon_tpm_crypto::hash::HashInstance::new(TpmiAlgHash::Sha512)
        .expect("SHA-512 must be supported");

    for p in &n.params {
        match p {
            NegotiationParam::Challenge => {
                let iter = SingletonIoSlice::new(&n.challenge).map_infallible_err::<CryptoError>();
                sha.update(iter).map_err(AttestationError::Crypto)?;
            }
            #[allow(irrefutable_let_patterns)]
            NegotiationParam::EcPublicKeyBytes => {
                let iter_x =
                    SingletonIoSlice::new(&pub_key.x.buffer).map_infallible_err::<CryptoError>();
                sha.update(iter_x).map_err(AttestationError::Crypto)?;
                let iter_y =
                    SingletonIoSlice::new(&pub_key.y.buffer).map_infallible_err::<CryptoError>();
                sha.update(iter_y).map_err(AttestationError::Crypto)?;
            }
        }
    }

    let res = sha.finalize().map_err(AttestationError::Crypto)?;
    let cocoon_tpm_tpm2_interface::TpmtHa::Sha512(buf) = res else {
        unreachable!();
    };
    try_to_vec(&buf).or(Err(AttestationError::VecAlloc))
}

#[cfg(test)]
mod tests {
    use super::*;
    use alloc::vec;
    use cocoon_tpm_tpm2_interface::{Tpm2bEccParameter, TpmBuffer};

    fn make_ecc_point(x: &[u8], y: &[u8]) -> TpmsEccPoint<'static> {
        TpmsEccPoint {
            x: Tpm2bEccParameter {
                buffer: TpmBuffer::Owned(x.to_vec()),
            },
            y: Tpm2bEccParameter {
                buffer: TpmBuffer::Owned(y.to_vec()),
            },
        }
    }

    mod negotiation_hash {
        use super::*;

        // The challenge size is server-dictated; 48 is an arbitrary choice.
        const CHALLENGE_LEN: usize = 48;
        // P-521 coordinate size.
        const COORD_LEN: usize = 66;

        /// hash() feeds NegotiationParams into SHA-512 in the order they
        /// appear in `response.params`. Verify that [Challenge, EcPublicKeyBytes]
        /// and [EcPublicKeyBytes, Challenge] each produce the correct digest and
        /// that the two digests differ — the server-negotiated ordering must
        /// affect the resulting attestation evidence.
        #[test]
        fn hash_respects_param_ordering() {
            let challenge = vec![0xdd; CHALLENGE_LEN];
            let x = vec![0x10; COORD_LEN];
            let y = vec![0x20; COORD_LEN];
            let pub_key = make_ecc_point(&x, &y);

            let response_chal_first = NegotiationResponse {
                challenge: challenge.clone(),
                params: vec![
                    NegotiationParam::Challenge,
                    NegotiationParam::EcPublicKeyBytes,
                ],
            };
            let response_key_first = NegotiationResponse {
                challenge: challenge.clone(),
                params: vec![
                    NegotiationParam::EcPublicKeyBytes,
                    NegotiationParam::Challenge,
                ],
            };

            let result_chal_first = hash(&response_chal_first, &pub_key).unwrap();
            let result_key_first = hash(&response_key_first, &pub_key).unwrap();

            // Reference digests of the fixed test inputs, computed
            // independently of this code with Python's hashlib:
            // sha512(challenge || x || y) and sha512(x || y || challenge).
            const EXPECTED_CHALLENGE_FIRST: [u8; 64] = [
                0xc2, 0x43, 0xa5, 0x4e, 0xd1, 0xca, 0xfc, 0xcd, 0x87, 0xf3, 0x2e, 0x3c, 0x2b, 0x34,
                0x09, 0x16, 0x2d, 0x14, 0x57, 0xf8, 0xcd, 0x24, 0x53, 0x78, 0xdc, 0xfb, 0x8b, 0xaa,
                0x29, 0x46, 0x69, 0xd0, 0x41, 0x84, 0x3e, 0x47, 0x8b, 0x9b, 0xfb, 0x6f, 0x6e, 0x99,
                0x64, 0x9e, 0x8b, 0xcb, 0xc8, 0xad, 0x3c, 0xe5, 0xa3, 0xba, 0x11, 0x40, 0xdc, 0xac,
                0xd1, 0x0d, 0x39, 0x1a, 0xce, 0x22, 0x20, 0x7b,
            ];
            const EXPECTED_KEY_FIRST: [u8; 64] = [
                0xbd, 0x35, 0x0b, 0x25, 0x3f, 0xae, 0xad, 0xc5, 0xff, 0x36, 0xfe, 0x96, 0x23, 0x6b,
                0x88, 0x3f, 0x28, 0x0f, 0xc0, 0x10, 0xf3, 0x87, 0x86, 0xc0, 0xc6, 0x53, 0x1e, 0xda,
                0xa3, 0xb9, 0xfe, 0x77, 0x97, 0xd1, 0xc0, 0xbc, 0x12, 0xa1, 0xe5, 0x5a, 0x1e, 0x6c,
                0x72, 0x13, 0x52, 0x93, 0x9a, 0x48, 0x62, 0x73, 0xa6, 0xc4, 0xb4, 0xec, 0x75, 0xeb,
                0xc9, 0x9f, 0xe0, 0xc6, 0x96, 0x51, 0xb6, 0x11,
            ];

            assert_eq!(result_chal_first, EXPECTED_CHALLENGE_FIRST);
            assert_eq!(result_key_first, EXPECTED_KEY_FIRST);

            assert_ne!(result_chal_first, result_key_first);
        }
    }
}

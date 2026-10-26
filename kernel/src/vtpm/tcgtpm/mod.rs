// SPDX-License-Identifier: MIT
//
// Copyright (C) 2023 IBM
// Copyright (c) 2026 Red Hat, Inc.
//
// Author: Claudio Carvalho <cclaudio@linux.ibm.com>

//! Virtual TPM backend using libtpms TPMLIB_* API.

mod persistence;
use persistence::{ManufacturedMarker, Marker as _};

pub mod ek_templates;
mod tss;

extern crate alloc;

use alloc::vec::Vec;

use core::sync::atomic::{AtomicU8, Ordering};
use libtcgtpm::bindings::{
    TPMLIB_ChooseTPMVersion, TPMLIB_MainInit, TPMLIB_Process, TPMLIB_RegisterCallbacks,
    TPMLIB_SetState, TPMLIB_StateType_TPMLIB_STATE_PERMANENT,
    TPMLIB_TPMVersion_TPMLIB_TPM_VERSION_2, TPMLIB_WasManufactured, TPM_MODIFIER_INDICATOR,
    TPM_RESULT, libtpms_callbacks,
};

unsafe extern "C" {
    fn free(ptr: *mut u8);
}

use crate::{
    protocols::{errors::SvsmReqError, vtpm::TpmPlatformCommand},
    types::PAGE_SIZE,
    vtpm::{
        TcgTpmSimulatorInterface, VtpmInterface, VtpmProtocolInterface,
        tcgtpm::ek_templates::DEFAULT_PUBLIC_AREA,
    },
};

const TPM_SUCCESS: TPM_RESULT = 0;

static CURRENT_LOCALITY: AtomicU8 = AtomicU8::new(0);

unsafe extern "C" fn locality_callback(
    locality: *mut TPM_MODIFIER_INDICATOR,
    _tpm_number: u32,
) -> TPM_RESULT {
    // SAFETY: caller (libtpms) provides a valid pointer
    unsafe { *locality = CURRENT_LOCALITY.load(Ordering::Relaxed) as u32 };
    TPM_SUCCESS
}

fn register_callbacks() -> Result<(), SvsmReqError> {
    let mut cbs = libtpms_callbacks {
        sizeOfStruct: core::mem::size_of::<libtpms_callbacks>() as i32,
        tpm_nvram_init: None,
        tpm_nvram_loaddata: None,
        tpm_nvram_storedata: None,
        tpm_nvram_deletename: None,
        tpm_io_init: None,
        tpm_io_getlocality: Some(locality_callback),
        tpm_io_getphysicalpresence: None,
    };

    // SAFETY: FFI call with valid struct pointer.
    let rc = unsafe { TPMLIB_RegisterCallbacks(&raw mut cbs) };
    if rc != TPM_SUCCESS {
        log::error!("TPMLIB_RegisterCallbacks failed rc={rc}");
        return Err(SvsmReqError::incomplete());
    }
    Ok(())
}

#[derive(Debug, Clone, Default)]
pub struct TcgTpm {
    is_powered_on: bool,
    ekpub: Option<Vec<u8>>,
}

impl TcgTpm {
    pub const fn new() -> TcgTpm {
        TcgTpm {
            is_powered_on: false,
            ekpub: None,
        }
    }
}

const TPM_CMDS_SUPPORTED: &[TpmPlatformCommand] = &[TpmPlatformCommand::SendCommand];

impl VtpmProtocolInterface for TcgTpm {
    fn get_supported_commands(&self) -> &[TpmPlatformCommand] {
        TPM_CMDS_SUPPORTED
    }
}

pub const TPM_BUFFER_MAX_SIZE: usize = PAGE_SIZE;

impl TcgTpmSimulatorInterface for TcgTpm {
    fn send_tpm_command(&self, command: &[u8], locality: u8) -> Result<Vec<u8>, SvsmReqError> {
        if !self.is_powered_on {
            return Err(SvsmReqError::invalid_request());
        }
        if command.len() > TPM_BUFFER_MAX_SIZE {
            return Err(SvsmReqError::invalid_parameter());
        }

        CURRENT_LOCALITY.store(locality, Ordering::Relaxed);

        let request_p = command.as_ptr() as *mut u8;
        let request_size = command.len() as u32;

        let mut response_p: *mut u8 = core::ptr::null_mut();
        let mut response_size: u32 = 0;
        let mut response_bufsize: u32 = 0;

        // SAFETY: FFI call. TPMLIB_Process manages its own response buffer
        // via realloc. The locality callback provides the locality value.
        let rc = unsafe {
            TPMLIB_Process(
                &raw mut response_p,
                &raw mut response_size,
                &raw mut response_bufsize,
                request_p,
                request_size,
            )
        };

        if rc != TPM_SUCCESS || response_p.is_null() || response_size == 0 {
            return Err(SvsmReqError::invalid_request());
        }

        // SAFETY: TPMLIB_Process allocated response_p via malloc/realloc
        // with at least response_size bytes.
        let response =
            unsafe { core::slice::from_raw_parts(response_p, response_size as usize) }.to_vec();

        // SAFETY: response_p was allocated by libtpms via malloc/realloc.
        unsafe { free(response_p) };

        Ok(response)
    }

    fn signal_poweron(&mut self, only_reset: bool) -> Result<(), SvsmReqError> {
        if only_reset && !self.is_powered_on {
            return Err(SvsmReqError::invalid_request());
        }
        // TPMLIB_MainInit already handled power-on and reset during init.
        // This is kept for interface compatibility.
        Ok(())
    }

    fn signal_nvon(&self) -> Result<(), SvsmReqError> {
        // TPMLIB_MainInit already signaled NV availability.
        Ok(())
    }
}

impl VtpmInterface for TcgTpm {
    fn get_ekpub(&mut self) -> Result<Vec<u8>, SvsmReqError> {
        if self.ekpub.is_none() {
            self.ekpub = Some(tss::create_ek(self, &DEFAULT_PUBLIC_AREA[..])?);
        }
        self.ekpub.clone().ok_or_else(SvsmReqError::invalid_request)
    }

    fn is_powered_on(&self) -> bool {
        self.is_powered_on
    }

    fn init(&mut self) -> Result<(), SvsmReqError> {
        // SAFETY: FFI call, no parameters.
        let rc = unsafe { TPMLIB_ChooseTPMVersion(TPMLIB_TPMVersion_TPMLIB_TPM_VERSION_2) };
        if rc != TPM_SUCCESS {
            log::error!("TPMLIB_ChooseTPMVersion failed rc={rc}");
            return Err(SvsmReqError::incomplete());
        }

        register_callbacks()?;

        // If a previous manufacture completed successfully, load the
        // persisted NV state and feed it to libtpms so that MainInit
        // skips manufacturing and restores the existing TPM state.
        if ManufacturedMarker::exists()? {
            if let Some(nv_data) = Self::load_nv_state() {
                log::info!("VTPM: restoring NV state from persistent storage");
                // SAFETY: FFI call with valid buffer pointer and length.
                let rc = unsafe {
                    TPMLIB_SetState(
                        TPMLIB_StateType_TPMLIB_STATE_PERMANENT,
                        nv_data.as_ptr(),
                        nv_data.len() as u32,
                    )
                };
                if rc != TPM_SUCCESS {
                    log::error!("TPMLIB_SetState(PERMANENT) failed rc={rc}");
                    return Err(SvsmReqError::incomplete());
                }
            }
        }

        // SAFETY: FFI call. MainInit handles NV enable, manufacture (if
        // needed), power-on, reset, and NV-available signaling internally.
        let rc = unsafe { TPMLIB_MainInit() };
        if rc != TPM_SUCCESS {
            log::error!("TPMLIB_MainInit failed rc={rc}");
            return Err(SvsmReqError::incomplete());
        }

        // SAFETY: FFI call, no parameters.
        let was_manufactured = unsafe { TPMLIB_WasManufactured() } != 0;
        if was_manufactured {
            log::info!("VTPM: manufactured new NV state");
            ManufacturedMarker::write()?;
        }

        self.is_powered_on = true;
        log::info!("VTPM: libtpms TPM 2.0 initialized");

        Ok(())
    }
}

impl TcgTpm {
    #[cfg(feature = "persistence")]
    fn load_nv_state() -> Option<Vec<u8>> {
        use crate::crt::TpmInode;
        use crate::persistence::{Inode, persistence_available, persistence_read_inode_sync};

        if !persistence_available() {
            return None;
        }
        match persistence_read_inode_sync(TpmInode::NvChip.inode()) {
            Ok(data) => data.map(|z| z.to_vec()),
            Err(e) => {
                log::error!("VTPM: failed to load NV state: {e:?}");
                None
            }
        }
    }

    #[cfg(not(feature = "persistence"))]
    fn load_nv_state() -> Option<Vec<u8>> {
        None
    }
}

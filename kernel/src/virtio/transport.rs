// SPDX-License-Identifier: MIT OR Apache-2.0
//
// Copyright (c) 2026 Red Hat, Inc.
//
// Author: Oliver Steffen <osteffen@redhat.com>

//! VirtIO MMIO transport with pluggable MMIO access.
//!
//! This module provides [`SvsmMmioTransport`], a reimplementation of the
//! upstream virtio-drivers [`MmioTransport`] that routes all register access
//! through the [`MmioAccess`] trait. This allows platforms that cannot use
//! regular volatile memory accesses (e.g. AMD SEV-SNP without a `#VC`
//! handler) to provide their own MMIO mechanism.
//!
//! The implementation is kept as close to the upstream `MmioTransport` as
//! possible. The only differences are in the actual MMIO read/write calls,
//! which go through `M::mmio_read()`/`M::mmio_write()` instead of
//! safe-mmio's volatile `read()`/`write()`.
//!
//! [`MmioTransport`]: virtio_drivers::transport::mmio::MmioTransport

use core::fmt::Debug;
use core::marker::PhantomData;
use core::mem::{align_of, size_of};
use core::ptr::NonNull;

use safe_mmio::fields::{ReadPure, ReadPureWrite, WriteOnly};
use safe_mmio::{SharedMmioPointer, UniqueMmioPointer, field, field_shared};
use virtio_drivers::transport::mmio::VirtIOHeader;
use virtio_drivers::transport::mmio::{MmioError, MmioVersion};
use virtio_drivers::transport::{DeviceStatus, DeviceType, InterruptStatus, Transport};
use virtio_drivers::{Error, PAGE_SIZE, PhysAddr};
use zerocopy::{FromBytes, Immutable, IntoBytes};

const MAGIC_VALUE: u32 = 0x7472_6976;
const CONFIG_SPACE_OFFSET: usize = 0x100;
const PAGE_SIZE_PHYS: PhysAddr = PAGE_SIZE as PhysAddr;

/// Trait for performing MMIO register reads and writes.
///
/// This abstracts the mechanism used to access MMIO registers, allowing
/// platforms that cannot use regular volatile memory accesses (e.g. AMD
/// SEV-SNP without a `#VC` handler) to provide their own implementation.
pub trait MmioAccess: Debug {
    /// # Safety
    /// `src` must point to a valid, properly aligned MMIO register.
    unsafe fn mmio_read<T: FromBytes>(src: *const T) -> T;

    /// # Safety
    /// `dst` must point to a valid, properly aligned MMIO register.
    unsafe fn mmio_write<T: IntoBytes + Immutable>(dst: *mut T, value: T);

    fn read_pure<T: FromBytes + IntoBytes>(x: &SharedMmioPointer<'_, ReadPure<T>>) -> T {
        x.read()
    }

    fn read_pure_write<T: FromBytes + IntoBytes>(x: &SharedMmioPointer<'_, ReadPureWrite<T>>) -> T {
        x.read()
    }

    fn write_only<T: IntoBytes + Immutable>(x: &mut UniqueMmioPointer<'_, WriteOnly<T>>, v: T) {
        x.write(v)
    }

    fn write_pure_write<T: IntoBytes + Immutable>(
        x: &mut UniqueMmioPointer<'_, ReadPureWrite<T>>,
        v: T,
    ) {
        x.write(v)
    }
}

/// VirtIO MMIO transport with pluggable MMIO access mechanism.
///
/// This is a reimplementation of the virtio-drivers `MmioTransport` that
/// routes all register access through the [`MmioAccess`] trait instead of
/// using `safe-mmio` volatile reads/writes directly.
#[derive(Debug)]
pub struct SvsmMmioTransport<M: MmioAccess> {
    header: UniqueMmioPointer<'static, VirtIOHeader>,
    config_space: UniqueMmioPointer<'static, [u8]>,
    version: MmioVersion,
    device_type: DeviceType,
    _phantom: PhantomData<M>,
}

// SAFETY: `SvsmMmioTransport` only accesses MMIO registers through the
// `MmioAccess` trait. The MMIO region is externally synchronized (callers
// protect it with a lock), and `NonNull<u8>` is the only non-Send/Sync
// field — it is used solely as an address, not shared across threads.
unsafe impl<M: MmioAccess> Send for SvsmMmioTransport<M> {}
// SAFETY: `&SvsmMmioTransport` only reads MMIO registers (get_status,
// read_config_generation, read_config_space, device_type). Concurrent
// reads of MMIO status registers are safe.
unsafe impl<M: MmioAccess> Sync for SvsmMmioTransport<M> {}

impl<M: MmioAccess> SvsmMmioTransport<M> {
    /// Constructs a new VirtIO MMIO transport, or returns an error if the
    /// header reports an unsupported version.
    ///
    /// # Safety
    ///
    /// `header` must point to a properly aligned valid VirtIO MMIO region,
    /// which must remain valid for the lifetime of the returned transport.
    /// `mmio_size` must be the size of the full MMIO region including config
    /// space.
    pub unsafe fn new(header: NonNull<u8>, mmio_size: usize) -> Result<Self, MmioError> {
        let Some(config_space_size) = mmio_size.checked_sub(CONFIG_SPACE_OFFSET) else {
            return Err(MmioError::MmioRegionTooSmall);
        };
        let config_space = NonNull::slice_from_raw_parts(
            // SAFETY: CONFIG_SPACE_OFFSET is well within the range of `isize`. The memory range
            // must be within the bounds of the allocation, because our caller promised that
            // `header` was a valid VirtIO MMIO region including the config space after the header.
            unsafe { header.byte_add(CONFIG_SPACE_OFFSET) },
            config_space_size,
        );
        // SAFETY: The caller promises that the config space following the header is an MMIO region
        // valid for the transport's lifetime.
        let config_space = unsafe { UniqueMmioPointer::new(config_space) };

        // SAFETY: The caller promises that `header` is a properly aligned MMIO region valid for
        // the transport's lifetime.
        let header = unsafe { UniqueMmioPointer::new(header.cast::<VirtIOHeader>()) };

        let magic = M::read_pure(&field_shared!(header, magic));
        if magic != MAGIC_VALUE {
            return Err(MmioError::BadMagic(magic));
        }
        let device_id = M::read_pure(&field_shared!(header, device_id));
        let device_type = DeviceType::try_from(device_id).map_err(MmioError::InvalidDeviceID)?;
        let version: u32 = M::read_pure(&field_shared!(header, version));
        let version = MmioVersion::try_from(version)?;

        Ok(Self {
            header,
            version,
            device_type,
            config_space,
            _phantom: PhantomData,
        })
    }

    /// Gets the version of the VirtIO MMIO transport.
    pub fn version(&self) -> MmioVersion {
        self.version
    }

    /// Gets the vendor ID.
    pub fn vendor_id(&self) -> u32 {
        M::read_pure(&field_shared!(self.header, vendor_id))
    }
}

impl<M: MmioAccess> Transport for SvsmMmioTransport<M> {
    fn device_type(&self) -> DeviceType {
        self.device_type
    }

    fn read_device_features(&mut self) -> u64 {
        M::write_only(&mut field!(self.header, device_features_sel), 0);
        let mut device_features_bits: u64 =
            M::read_pure(&field_shared!(self.header, device_features)).into();
        M::write_only(&mut field!(self.header, device_features_sel), 1);
        device_features_bits +=
            (M::read_pure(&field_shared!(self.header, device_features)) as u64) << 32;
        device_features_bits
    }

    fn write_driver_features(&mut self, driver_features: u64) {
        M::write_only(&mut field!(self.header, driver_features_sel), 0);
        M::write_only(
            &mut field!(self.header, driver_features),
            driver_features as u32,
        );
        M::write_only(&mut field!(self.header, driver_features_sel), 1);
        M::write_only(
            &mut field!(self.header, driver_features),
            (driver_features >> 32) as u32,
        );
    }

    fn max_queue_size(&mut self, queue: u16) -> u32 {
        M::write_only(&mut field!(self.header, queue_sel), queue.into());
        M::read_pure(&field_shared!(self.header, queue_num_max))
    }

    fn notify(&mut self, queue: u16) {
        M::write_only(&mut field!(self.header, queue_notify), queue.into());
    }

    fn get_status(&self) -> DeviceStatus {
        M::read_pure_write(&field_shared!(self.header, status))
    }

    fn set_status(&mut self, status: DeviceStatus) {
        M::write_pure_write(&mut field!(self.header, status), status);
    }

    fn set_guest_page_size(&mut self, guest_page_size: u32) {
        match self.version {
            MmioVersion::Legacy => {
                M::write_only(
                    &mut field!(self.header, legacy_guest_page_size),
                    guest_page_size,
                );
            }
            MmioVersion::Modern => {
                // No-op, modern devices don't care.
            }
        }
    }

    fn requires_legacy_layout(&self) -> bool {
        match self.version {
            MmioVersion::Legacy => true,
            MmioVersion::Modern => false,
        }
    }

    fn queue_set(
        &mut self,
        queue: u16,
        size: u32,
        descriptors: PhysAddr,
        driver_area: PhysAddr,
        device_area: PhysAddr,
    ) {
        match self.version {
            MmioVersion::Legacy => {
                let align = PAGE_SIZE as u32;
                let pfn = (descriptors / PAGE_SIZE_PHYS).try_into().unwrap();
                assert_eq!(u64::from(pfn) * PAGE_SIZE_PHYS, descriptors);
                M::write_only(&mut field!(self.header, queue_sel), queue.into());
                M::write_only(&mut field!(self.header, queue_num), size);
                M::write_only(&mut field!(self.header, legacy_queue_align), align);
                M::write_pure_write(&mut field!(self.header, legacy_queue_pfn), pfn);
            }
            MmioVersion::Modern => {
                M::write_only(&mut field!(self.header, queue_sel), queue.into());
                M::write_only(&mut field!(self.header, queue_num), size);
                M::write_only(&mut field!(self.header, queue_desc_low), descriptors as u32);
                M::write_only(
                    &mut field!(self.header, queue_desc_high),
                    (descriptors >> 32) as u32,
                );
                M::write_only(
                    &mut field!(self.header, queue_driver_low),
                    driver_area as u32,
                );
                M::write_only(
                    &mut field!(self.header, queue_driver_high),
                    (driver_area >> 32) as u32,
                );
                M::write_only(
                    &mut field!(self.header, queue_device_low),
                    device_area as u32,
                );
                M::write_only(
                    &mut field!(self.header, queue_device_high),
                    (device_area >> 32) as u32,
                );
                M::write_pure_write(&mut field!(self.header, queue_ready), 1);
            }
        }
    }

    fn queue_unset(&mut self, queue: u16) {
        match self.version {
            MmioVersion::Legacy => {
                M::write_only(&mut field!(self.header, queue_sel), queue.into());
                M::write_only(&mut field!(self.header, queue_num), 0);
                M::write_only(&mut field!(self.header, legacy_queue_align), 0);
                M::write_pure_write(&mut field!(self.header, legacy_queue_pfn), 0);
            }
            MmioVersion::Modern => {
                M::write_only(&mut field!(self.header, queue_sel), queue.into());
                M::write_pure_write(&mut field!(self.header, queue_ready), 0);
                // Wait until we read the same value back, to ensure synchronisation (see 4.2.2.2).
                while M::read_pure_write(&field_shared!(self.header, queue_ready)) != 0 {}
                M::write_only(&mut field!(self.header, queue_num), 0);
                M::write_only(&mut field!(self.header, queue_desc_low), 0);
                M::write_only(&mut field!(self.header, queue_desc_high), 0);
                M::write_only(&mut field!(self.header, queue_driver_low), 0);
                M::write_only(&mut field!(self.header, queue_driver_high), 0);
                M::write_only(&mut field!(self.header, queue_device_low), 0);
                M::write_only(&mut field!(self.header, queue_device_high), 0);
            }
        }
    }

    fn queue_used(&mut self, queue: u16) -> bool {
        M::write_only(&mut field!(self.header, queue_sel), queue.into());
        match self.version {
            MmioVersion::Legacy => {
                M::read_pure_write(&field_shared!(self.header, legacy_queue_pfn)) != 0
            }
            MmioVersion::Modern => {
                M::read_pure_write(&field_shared!(self.header, queue_ready)) != 0
            }
        }
    }

    fn ack_interrupt(&mut self) -> InterruptStatus {
        let interrupt = M::read_pure(&field_shared!(self.header, interrupt_status));
        if interrupt != 0 {
            M::write_only(&mut field!(self.header, interrupt_ack), interrupt);
            InterruptStatus::from_bits_truncate(interrupt)
        } else {
            InterruptStatus::empty()
        }
    }

    fn read_config_generation(&self) -> u32 {
        M::read_pure(&field_shared!(self.header, config_generation))
    }

    fn read_config_space<T: FromBytes + IntoBytes>(&self, offset: usize) -> Result<T, Error> {
        assert!(
            align_of::<T>() <= 4,
            "Driver expected config space alignment of {} bytes, but VirtIO only guarantees 4 byte alignment.",
            align_of::<T>()
        );
        assert!(offset.is_multiple_of(align_of::<T>()));

        if self.config_space.len() < offset + size_of::<T>() {
            Err(Error::ConfigSpaceTooSmall)
        } else {
            // SAFETY: The caller of `SvsmMmioTransport::new` guaranteed that the header pointer
            // was valid, including the config space. We have checked that the value is properly
            // aligned for `T` and within the bounds of the config space.
            unsafe {
                let ptr = self.config_space.ptr().cast::<T>().byte_add(offset);
                Ok(M::mmio_read(ptr))
            }
        }
    }

    fn write_config_space<T: IntoBytes + Immutable>(
        &mut self,
        offset: usize,
        value: T,
    ) -> Result<(), Error> {
        assert!(
            align_of::<T>() <= 4,
            "Driver expected config space alignment of {} bytes, but VirtIO only guarantees 4 byte alignment.",
            align_of::<T>()
        );
        assert!(offset.is_multiple_of(align_of::<T>()));

        if self.config_space.len() < offset + size_of::<T>() {
            Err(Error::ConfigSpaceTooSmall)
        } else {
            // SAFETY: The caller of `SvsmMmioTransport::new` guaranteed that the header pointer
            // was valid, including the config space. We have checked that the value is properly
            // aligned for `T` and within the bounds of the config space.
            unsafe {
                let ptr = self.config_space.ptr_nonnull().cast::<T>().byte_add(offset);
                M::mmio_write(ptr.as_ptr(), value);
            }
            Ok(())
        }
    }
}

impl<M: MmioAccess> Drop for SvsmMmioTransport<M> {
    fn drop(&mut self) {
        // Reset the device when the transport is dropped.
        self.set_status(DeviceStatus::empty());
    }
}

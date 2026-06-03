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

    fn read_pure_write<T: FromBytes + IntoBytes>(
        x: &SharedMmioPointer<'_, ReadPureWrite<T>>,
    ) -> T {
        x.read()
    }

    fn write_only<T: IntoBytes + Immutable>(
        x: &mut UniqueMmioPointer<'_, WriteOnly<T>>,
        v: T,
    ) {
        x.write(v)
    }

    fn write_pure_write<T: IntoBytes + Immutable>(
        x: &mut UniqueMmioPointer<'_, ReadPureWrite<T>>,
        v: T,
    ) {
        x.write(v)
    }
}

// MMIO Device Register Interface, both legacy and modern.
//
// This is a local copy of `virtio_drivers::transport::mmio::VirtIOHeader`
// with `pub` fields, so that safe-mmio `field!`/`field_shared!` macros can
// be used from outside the virtio-drivers crate. The upstream struct has
// private fields, which prevents external use of these macros.
//
// TODO: Propose making the upstream fields `pub` so this copy can be removed.
//
// Ref: VirtIO spec 4.2.2 MMIO Device Register Layout and 4.2.4 Legacy interface
#[derive(Debug)]
#[repr(C)]
struct VirtIOHeader {
    /// Magic value
    pub magic: ReadPure<u32>,

    /// Device version number
    ///
    /// Legacy device returns value 0x1.
    pub version: ReadPure<u32>,

    /// Virtio Subsystem Device ID
    pub device_id: ReadPure<u32>,

    /// Virtio Subsystem Vendor ID
    pub vendor_id: ReadPure<u32>,

    /// Flags representing features the device supports
    pub device_features: ReadPure<u32>,

    /// Device (host) features word selection
    pub device_features_sel: WriteOnly<u32>,

    /// Reserved
    __r1: [u32; 2],

    /// Flags representing device features understood and activated by the driver
    pub driver_features: WriteOnly<u32>,

    /// Activated (guest) features word selection
    pub driver_features_sel: WriteOnly<u32>,

    /// Guest page size
    ///
    /// The driver writes the guest page size in bytes to the register during
    /// initialization, before any queues are used. This value should be a
    /// power of 2 and is used by the device to calculate the Guest address
    /// of the first queue page (see QueuePFN).
    pub legacy_guest_page_size: WriteOnly<u32>,

    /// Reserved
    __r2: u32,

    /// Virtual queue index
    ///
    /// Writing to this register selects the virtual queue that the following
    /// operations on the QueueNumMax, QueueNum, QueueAlign and QueuePFN
    /// registers apply to. The index number of the first queue is zero (0x0).
    pub queue_sel: WriteOnly<u32>,

    /// Maximum virtual queue size
    ///
    /// Reading from the register returns the maximum size of the queue the
    /// device is ready to process or zero (0x0) if the queue is not available.
    /// This applies to the queue selected by writing to QueueSel and is
    /// allowed only when QueuePFN is set to zero (0x0), so when the queue is
    /// not actively used.
    pub queue_num_max: ReadPure<u32>,

    /// Virtual queue size
    ///
    /// Queue size is the number of elements in the queue. Writing to this
    /// register notifies the device what size of the queue the driver will use.
    /// This applies to the queue selected by writing to QueueSel.
    pub queue_num: WriteOnly<u32>,

    /// Used Ring alignment in the virtual queue
    ///
    /// Writing to this register notifies the device about alignment boundary
    /// of the Used Ring in bytes. This value should be a power of 2 and
    /// applies to the queue selected by writing to QueueSel.
    pub legacy_queue_align: WriteOnly<u32>,

    /// Guest physical page number of the virtual queue
    ///
    /// Writing to this register notifies the device about location of the
    /// virtual queue in the Guest's physical address space. This value is
    /// the index number of a page starting with the queue Descriptor Table.
    /// Value zero (0x0) means physical address zero (0x00000000) and is illegal.
    /// When the driver stops using the queue it writes zero (0x0) to this
    /// register. Reading from this register returns the currently used page
    /// number of the queue, therefore a value other than zero (0x0) means that
    /// the queue is in use. Both read and write accesses apply to the queue
    /// selected by writing to QueueSel.
    pub legacy_queue_pfn: ReadPureWrite<u32>,

    /// new interface only
    pub queue_ready: ReadPureWrite<u32>,

    /// Reserved
    __r3: [u32; 2],

    /// Queue notifier
    pub queue_notify: WriteOnly<u32>,

    /// Reserved
    __r4: [u32; 3],

    /// Interrupt status
    pub interrupt_status: ReadPure<u32>,

    /// Interrupt acknowledge
    pub interrupt_ack: WriteOnly<u32>,

    /// Reserved
    __r5: [u32; 2],

    /// Device status
    ///
    /// Reading from this register returns the current device status flags.
    /// Writing non-zero values to this register sets the status flags,
    /// indicating the OS/driver progress. Writing zero (0x0) to this register
    /// triggers a device reset. The device sets QueuePFN to zero (0x0) for
    /// all queues in the device. Also see 3.1 Device Initialization.
    pub status: ReadPureWrite<DeviceStatus>,

    /// Reserved
    __r6: [u32; 3],

    // new interface only since here
    pub queue_desc_low: WriteOnly<u32>,
    pub queue_desc_high: WriteOnly<u32>,

    /// Reserved
    __r7: [u32; 2],

    pub queue_driver_low: WriteOnly<u32>,
    pub queue_driver_high: WriteOnly<u32>,

    /// Reserved
    __r8: [u32; 2],

    pub queue_device_low: WriteOnly<u32>,
    pub queue_device_high: WriteOnly<u32>,

    /// Reserved
    __r9: [u32; 21],

    pub config_generation: ReadPure<u32>,
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

// SPDX-License-Identifier: MIT
//
// Copyright (c) 2024 Red Hat, Inc.
//
// Author: Oliver Steffen <osteffen@redhat.com>

use crate::locking::SpinLock;
use core::{
    alloc::Layout,
    cell::OnceCell,
    ptr::{addr_of, NonNull},
};
use zerocopy::{FromBytes, Immutable, IntoBytes};

use crate::{
    address::{PhysAddr, VirtAddr},
    cpu::{self, percpu::this_cpu},
    mm::{page_visibility::*, *},
};

//use linked_list_allocator::Heap;

const SHARED_MEMORY_SIZE: usize = 64 * SIZE_1K;

struct SharedMemory {
    _mem: SharedBox<[u8; SHARED_MEMORY_SIZE]>,
    pub allocator: linked_list_allocator::Heap,
}

impl SharedMemory {
    pub fn new() -> Self {
        log::info!("new shared");
        let mem = SharedBox::<[u8; SHARED_MEMORY_SIZE]>::try_new_zeroed().unwrap();
        let prt = mem.addr().as_mut_ptr::<u8>();
        let alloc = unsafe { linked_list_allocator::Heap::new(prt, SHARED_MEMORY_SIZE) };
        log::info!("new shared alloc");
        Self {
            _mem: mem,
            allocator: alloc,
        }
    }
}

static SHARED_MEM: SpinLock<OnceCell<SharedMemory>> = SpinLock::new(OnceCell::new());

pub fn virtio_init() {
    SHARED_MEM.lock().get_or_init(|| SharedMemory::new());
}

#[derive(Debug)]
pub struct SvsmHal;

/// Implementation of virtio-drivers MMIO hardware abstraction for AMD SEV-SNP
/// in the Coconut-SVSM context. Due to missing #VC handler for MMIO, use ghcb exits
/// instead.
///
/// SAFETY: Complies with the safety requirements of the virtio_drivers::Hal trait.
unsafe impl virtio_drivers::Hal for SvsmHal {
    /// Allocates and zeroes the given number of contiguous physical pages of DMA memory for VirtIO
    /// use.
    ///
    /// Limitation: `pages == 1` required.
    fn dma_alloc(
        pages: usize,
        _direction: virtio_drivers::BufferDirection,
    ) -> (virtio_drivers::PhysAddr, NonNull<u8>) {
        // TODO: allow more than one page.
        //       This currently works, becasue in "modern" virtio mode the crate only allocates
        //       one page at a time.

        log::info!("dma_alloc");
        let layout = Layout::from_size_align(PAGE_SIZE * pages, PAGE_SIZE).unwrap();
        let m = SHARED_MEM
            .lock()
            .get_mut()
            .unwrap()
            .allocator
            .allocate_first_fit(layout)
            .unwrap();
        unsafe {
            m.as_ptr().write_bytes(0, PAGE_SIZE * pages);
        }

        let pa = virt_to_phys(VirtAddr::from(m.as_ptr()));

        (pa.into(), m)
    }

    /// Deallocates the given contiguous physical DMA memory pages.
    ///
    /// # Safety
    ///
    /// The memory must have been allocated by `dma_alloc` on the same `Hal` implementation, and not
    /// yet deallocated. `pages` must be the same number passed to `dma_alloc` originally, and both
    /// `paddr` and `vaddr` must be the values returned by `dma_alloc`.
    ///
    /// Limitation: `pages == 1` required.
    unsafe fn dma_dealloc(
        _paddr: virtio_drivers::PhysAddr,
        vaddr: NonNull<u8>,
        pages: usize,
    ) -> i32 {
        log::info!("dma_dealloc");
        let layout = Layout::from_size_align(PAGE_SIZE * pages, PAGE_SIZE).unwrap();
        unsafe {
            SHARED_MEM
                .lock()
                .get_mut()
                .unwrap()
                .allocator
                .deallocate(vaddr, layout);
        }

        0
    }

    /// Converts a physical address used for MMIO to a virtual address which the driver can access.
    /// NOT IMPLEMENTED - only required for PCI transport, which is not used in Coconut.
    unsafe fn mmio_phys_to_virt(_paddr: virtio_drivers::PhysAddr, _size: usize) -> NonNull<u8> {
        todo!()
    }

    /// Shares the given memory range with the device, and returns the physical address that the
    /// device can use to access it.
    ///
    /// # Safety
    ///
    /// The buffer must be a valid pointer to a non-empty memory range which will not be accessed by
    /// any other thread for the duration of this method call.
    ///
    /// Limitation: `buffer.len() <= PAGE_SIZE`
    unsafe fn share(
        buffer: NonNull<[u8]>,
        direction: virtio_drivers::BufferDirection,
    ) -> virtio_drivers::PhysAddr {
        // TODO: allow more than one page

        let layout = Layout::array::<u8>(buffer.len()).unwrap();
        let mut shared_page = SHARED_MEM
            .lock()
            .get_mut()
            .unwrap()
            .allocator
            .allocate_first_fit(layout)
            .unwrap();

        if direction == virtio_drivers::BufferDirection::DriverToDevice {
            let src = buffer.as_ptr().cast::<u8>();
            let dst = unsafe { shared_page.as_mut() };

            // SAFETY: Assume `buffer` (== `src`) is valid because it is supplied by the vritio-drivers crate.
            //         `dst` is valid and has the correct size because we allocated it
            unsafe {
                core::ptr::copy_nonoverlapping(src, dst, buffer.len());
            }
        }

        let pa = virt_to_phys(VirtAddr::from(shared_page.as_ptr()));

        log::info!(
            "share   [{:016x} + {:04x}], Layout: {:04x}/{:04x} -> va: {:016x} pa: {:016x}",
            VirtAddr::from(buffer.as_ptr().cast::<u8>()),
            buffer.len(),
            layout.size(), layout.align(),
            VirtAddr::from(shared_page.as_ptr()),
            pa
        );

        // return pa of shared page
        pa.into()
    }

    /// Unshares the given memory range from the device and (if necessary) copies it back to the
    /// original buffer.
    ///
    /// # Safety
    ///
    /// The buffer must be a valid pointer to a non-empty memory range which will not be accessed by
    /// any other thread for the duration of this method call. The `paddr` must be the value
    /// previously returned by the corresponding `share` call.
    ///
    /// Limitation: `buffer.len() <= PAGE_SIZE`
    unsafe fn unshare(
        paddr: virtio_drivers::PhysAddr,
        buffer: NonNull<[u8]>,
        direction: virtio_drivers::BufferDirection,
    ) {
        let p = phys_to_virt(PhysAddr::from(paddr)).as_mut_ptr::<u8>();
        let shared_page = unsafe { NonNull::<u8>::new_unchecked(p) };

        if direction == virtio_drivers::BufferDirection::DeviceToDriver {
            let dst = buffer.as_ptr().cast::<u8>();
            let src = p;

            // SAFETY: Assume `buffer` is valid and can hold at leader `buffer.len()`
            //         because both are supplied by the vritio-drivers crate.
            //         We assterted that `src` holds at least `buffer.len()`.
            unsafe {
                core::ptr::copy_nonoverlapping(src, dst, buffer.len());
            }
        }

        let layout = Layout::array::<u8>(buffer.len()).unwrap();
        unsafe {
            SHARED_MEM
                .lock()
                .get_mut()
                .unwrap()
                .allocator
                .deallocate(shared_page, layout);
        }

        log::info!(
            "unshare [{:016x} + {:04x}], Layout: {:04x}/{:04x} -> va: {:016x} pa: {:016x}",
            VirtAddr::from(buffer.as_ptr().cast::<u8>()),
            buffer.len(),
            layout.size(), layout.align(),
            VirtAddr::from(shared_page.as_ptr()),
            PhysAddr::from(paddr)
        );
    }

    /// Performs memory mapped read from location of `src`. `src` itself is not modified,
    /// the value is returned instead.
    ///
    /// The default implementation performs a regular volatile_read. This method is intended
    /// to be overwritten in case MMIO memory needs to be accessed in a special way (for example AMD SEV-SNP).
    ///
    /// # Safety
    ///
    /// `src` must be properly alinged and reside at a readable memory address.
    unsafe fn mmio_read<T: FromBytes + Immutable>(src: &T) -> T {
        let paddr = this_cpu()
            .get_pgtable()
            .phys_addr(VirtAddr::from(addr_of!(*src)))
            .unwrap();

        cpu::percpu::current_ghcb()
            .mmio_read::<T>(paddr)
            .expect("GHCB MMIO Read failed")
    }

    /// Performs memory mapped write of `value` to the location of `dst`.
    ///
    /// The default implementation performs a regular volatile_write. This method is intended
    /// to be overwritten in case MMIO memory needs to be accessed in a special way (for example AMD SEV-SNP).
    ///
    /// # Safety
    ///
    /// `dst` must be properly alinged and reside at a writable memory address.
    unsafe fn mmio_write<T: IntoBytes + Immutable>(dst: &mut T, v: T) {
        let paddr = this_cpu()
            .get_pgtable()
            .phys_addr(VirtAddr::from(addr_of!(*dst)))
            .unwrap();

        cpu::percpu::current_ghcb()
            .mmio_write::<T>(paddr, &v)
            .expect("GHCB MMIO Write failed");
    }
}

// SPDX-License-Identifier: MIT OR Apache-2.0
//
// Copyright (c) 2024 Red Hat, Inc.
//
// Author: Oliver Steffen <osteffen@redhat.com>

use super::BlockDev;
use core::{cmp::min, slice, usize};

use crate::{
    address::{Address, PhysAddr}, cpu, error::SvsmError, types::PAGE_SIZE, utils::{page_align_up, MemoryRegion}
};

/// PFlash command codes
const WRITE_BYTE_CMD: u8 = 0x10;
// const BLOCK_ERASE_CMD: u8 = 0x20;
// const CLEAR_STATUS_CMD: u8 = 0x50;
// const READ_STATUS_CMD: u8 = 0x70;
// const READ_DEVID_CMD: u8 = 0x90;
// const BLOCK_ERASE_CONFIRM_CMD: u8 = 0xd0;
const READ_ARRAY_CMD: u8 = 0xff;

mod util {
    use crate::{
        address::PhysAddr,
        error::SvsmError,
        mm::{page_visibility::make_page_shared_2, PerCPUPageMappingGuard},
        sev::pvalidate,
        types::PageSize,
    };

    pub(crate) fn map_page_shared(pa: PhysAddr) -> Result<PerCPUPageMappingGuard, SvsmError> {
        let guard = PerCPUPageMappingGuard::create_4k(pa)?;
        pvalidate(
            guard.virt_region().start(),
            PageSize::Regular,
            crate::sev::PvalidateOp::Valid,
        )
        .expect("sd");
        make_page_shared_2(guard.virt_region().start(), pa);
        Ok(guard)
    }
}

struct Pflash {
    base_address: PhysAddr,
    size: usize,
}

impl Pflash {
    pub fn new(base_address: PhysAddr, size: usize) -> Self {
        assert!(base_address.is_page_aligned());
        assert!(size % PAGE_SIZE == 0);

        Pflash { base_address, size }
    }

    fn get_pa(&self, offset: usize) -> (PhysAddr, usize) {
        let pa = self.base_address.const_add(offset);
        (pa.page_align(), pa.page_offset())
    }

    fn read_page(&self, buf: &mut [u8], offset: usize) -> Result<usize, SvsmError> {
        let (page, poffs) = self.get_pa(offset);
        log::info!("  pr {page:016x}, poffs={poffs:04x}");

        let guard = util::map_page_shared(page)?;
        let data =
            unsafe { slice::from_raw_parts_mut(guard.virt_addr().as_mut_ptr::<u8>(), PAGE_SIZE) };

        let len = min(buf.len(), PAGE_SIZE - poffs);
        buf[0..len].copy_from_slice(&data[poffs..(poffs + len)]);
        Ok(len)
    }

    fn write_page(&self, buf: &[u8], offset: usize) -> Result<usize, SvsmError> {
        let (page, poffs) = self.get_pa(offset);
        log::info!("  pw {page:016x}, poffs={poffs:04x}");

        let _guard = util::map_page_shared(page)?;

        let len = min(buf.len(), PAGE_SIZE - poffs);
        for p in 0..len {
            let pa = page.const_add(poffs + p);
            let v = buf[p];
            log::info!("flash write buf[{len}] to pa {pa:016x}, v = {v:02x}");

            cpu::ghcb::current_ghcb()
                .mmio_write_byte(pa, WRITE_BYTE_CMD)
                .expect("MMIO");
            cpu::ghcb::current_ghcb()
                .mmio_write_byte(pa, v)
                .expect("MMIO");
        }
        cpu::ghcb::current_ghcb()
            .mmio_write_byte(page.const_add(poffs + len), READ_ARRAY_CMD)
            .expect("MMIO");
        Ok(len)
    }
}

impl BlockDev for Pflash {
    fn read(&self, buf: &mut [u8], offset: usize) -> Result<usize, SvsmError> {
        let mut current = min(offset, self.size);
        let mut len = buf.len();
        let mut bytes: usize = 0;
        let mut buf_offset = 0;

        while len > 0 {
            let page_end = min(page_align_up(current + 1), self.size);
            let page_len = min(page_end - current, len);
            let buf_end = buf_offset + page_len;

            if page_len == 0 {
                break;
            }
            log::info!(" rr buf[{buf_offset}..{buf_end}], current={current}");
            self.read_page(&mut buf[buf_offset..buf_end], current)?;

            buf_offset = buf_end;
            current += page_len;
            len -= page_len;
            bytes += page_len;
        }

        Ok(bytes)
    }

    fn write(&mut self, buf: &[u8], offset: usize) -> Result<usize, SvsmError> {
        let mut current = min(offset, self.size);
        let mut len = buf.len();
        let mut bytes: usize = 0;
        let mut buf_offset = 0;

        while len > 0 {
            let page_end = min(page_align_up(current + 1), self.size);
            let page_len = min(page_end - current, len);
            let buf_end = buf_offset + page_len;

            if page_len == 0 {
                break;
            }
            log::info!(" rr buf[{buf_offset}..{buf_end}], current={current}");
            self.write_page(&buf[buf_offset..buf_end], current)?;

            buf_offset = buf_end;
            current += page_len;
            len -= page_len;
            bytes += page_len;
        }

        Ok(bytes)
    }

    fn size(&self) -> usize {
        self.size
    }
}


fn test_drv(flash: MemoryRegion<PhysAddr>) {
    let mut d = Pflash::new(flash.start(), flash.end() - flash.start());

    let mut buf = [0u8; 32];
    let len = d.read(&mut buf, 0).expect("read failed");
    log::info!("drv read offset 0, res={len}: {:02x?}", buf);

    let wd = [0xccu8, 0xaa, 0xbb];
    let wlen = d.write(&wd, 0).expect("write failed");
    log::info!("drv write offset 0: len={wlen}");

    let len = d.read(&mut buf, 0).expect("read failed");
    log::info!("drv read offset 0, res={len}: {:02x?}", buf);

    {
        let wd = [0x55u8, 0x66, 0x77];
        let wlen = d.write(&wd, 0).expect("write failed");
        log::info!("drv write offset 0: len={wlen}");
    }
    {
        let mut buf2 = [0u8; 32];
        let len = d.read(&mut buf2, PAGE_SIZE - 16).expect("read failed");
        log::info!("drv read offset 0, res={len}: {:02x?}", buf2);
    }
}

pub fn test(flash: MemoryRegion<PhysAddr>) {
    log::info!(
        "MemoryRegion = {:016x} - {:016x}",
        flash.start(),
        flash.end()
    );
    test_drv(flash);
}

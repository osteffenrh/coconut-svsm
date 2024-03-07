// SPDX-License-Identifier: MIT OR Apache-2.0
//
// Copyright (c) 2024 Red Hat, Inc.
//
// Author: Oliver Steffen <osteffen@redhat.com>

use super::BlockDev;
extern crate alloc;
use core::cmp::min;

use crate::error::SvsmError;
use alloc::vec;
use alloc::vec::Vec;

#[derive(Debug)]
pub struct RamDisk {
    data: Vec<u8>,
}

impl RamDisk {
    pub fn new(size: usize) -> Self {
        RamDisk {
            data: vec![0u8; size],
        }
    }

    pub fn from_content(content: &Vec<u8>) -> Self {
        RamDisk {
            data: content.clone(),
        }
    }
}

impl BlockDev for RamDisk {
    fn read(&self, buf: &mut [u8], offset: usize) -> Result<usize, SvsmError> {
        let pos = min(offset, self.size());
        let len = min(buf.len(), self.size() - pos);
        buf[0..len].copy_from_slice(&self.data[pos..(pos + len)]);
        Ok(len)
    }

    fn write(&mut self, buf: &[u8], offset: usize) -> Result<usize, SvsmError> {
        let pos = min(offset, self.size());
        let len = min(buf.len(), self.size() - pos);
        self.data[pos..(pos + len)].copy_from_slice(&buf[0..len]);
        Ok(len)
    }

    fn size(&self) -> usize {
        self.data.len()
    }
}

#[cfg(test)]
mod tests {

    extern crate alloc;
    use crate::block::api::BlockDev;
    use crate::block::ramdisk::RamDisk;
    use alloc::vec;

    #[test]
    pub fn size() {
        let dev = RamDisk::new(123);
        assert_eq!(dev.size(), 123);

        let dev2 = RamDisk::new(321);
        assert_eq!(dev2.size(), 321);
    }

    #[test]
    pub fn write() {
        let buf = [1, 2, 3];
        let mut a = RamDisk::new(0x1000);
        assert_eq!(a.write(&buf, 0).unwrap(), 3);
    }

    #[test]
    pub fn write_from_slice() {
        let buf = [1, 2, 3, 4];
        let mut a = RamDisk::new(0x1000);
        assert_eq!(a.write(&buf[2..3], 0).unwrap(), 1);
    }

    #[test]
    pub fn write_out_of_bounds() {
        let buf = [1, 2, 3, 4];
        let mut a = RamDisk::new(0x1000);

        assert_eq!(a.write(&buf, a.size() - 1).unwrap(), 1);
        assert_eq!(a.write(&buf, a.size() + 1).unwrap(), 0);
    }

    #[test]
    pub fn read_out_of_bounds() {
        let a = RamDisk::from_content(&vec![0u8, 1, 2, 3, 4, 5]);
        let mut buf = [0u8; 4];

        // short read
        assert_eq!(a.read(&mut buf, a.size() - 2).unwrap(), 2);
        assert_eq!(buf[0..2], [4u8, 5]);

        // no data returned
        assert_eq!(a.read(&mut buf, a.size() + 2).unwrap(), 0);
    }

    #[test]
    pub fn read() {
        let a = RamDisk::from_content(&vec![0u8, 1, 2, 3, 4, 5]);
        {
            let mut buf = [0u8, 0, 0];
            assert_eq!(a.read(&mut buf, 0).unwrap(), buf.len());
            assert_eq!(buf, [0u8, 1, 2]);
        }
        {
            let mut buf = [0u8, 0, 0];
            assert_eq!(a.read(&mut buf, 3).unwrap(), buf.len());
            assert_eq!(buf, [3u8, 4, 5]);
        }
    }

    #[test]
    pub fn read_into_slice() {
        let a = RamDisk::from_content(&vec![0u8, 1, 2, 3, 4, 5]);
        let mut buf = [0u8; 10];
        let tgt = &mut buf[3..6];
        assert_eq!(a.read(tgt, 1).unwrap(), tgt.len());
        assert_eq!(*tgt, [1u8, 2, 3]);
        assert_eq!(buf, [0u8, 0, 0, 1, 2, 3, 0, 0, 0, 0]);
    }

    #[test]
    pub fn readback() {
        let mut a = RamDisk::from_content(&vec![0u8, 1, 2, 3, 4, 5, 6, 7, 8, 9]);

        let write_data = [20u8, 21u8, 22u8];

        assert_eq!(a.write(&write_data, 2).unwrap(), write_data.len());

        const EXPECT: [u8; 5] = [1u8, 20, 21, 22, 5];
        let mut read_data = [0u8; EXPECT.len()];
        assert_eq!(a.read(&mut read_data, 1).unwrap(), EXPECT.len());
        assert_eq!(read_data, EXPECT);
    }
}

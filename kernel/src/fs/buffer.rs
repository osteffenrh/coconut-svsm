// SPDX-License-Identifier: MIT
//
// Copyright (c) 2024 SUSE LLC
//
// Author: Joerg Roedel <jroedel@suse.de>

use crate::address::VirtAddr;
use crate::error::SvsmError;
use crate::fs::FsError;
use crate::mm::{copy_from_user, copy_to_user};
use core::cmp;

pub trait Buffer {
    /// Copy data from the buffer into a slice
    ///
    /// # Arguments
    ///
    /// - `buf`: Destination slice for data.
    /// - `offset`: Offset into the buffer to start copying from.
    ///
    /// # Returns
    ///
    /// A `usize` representing the number of bytes copied on success, or
    /// [`SvsmError`] on failure. Not that the content of `buf` is undefined on
    /// failure.
    fn read_buffer(&self, buf: &mut [u8], offset: usize) -> Result<usize, SvsmError>;

    /// Copy data from a slice into the Buffer
    ///
    /// # Arguments
    ///
    /// - `buf`: Source slice for data.
    /// - `offset`: Offset into the buffer to start copying to.
    ///
    /// # Returns
    ///
    /// A `usize` representing the number of bytes copied on success, or
    /// [`SvsmError`] on failure.
    fn write_buffer(&mut self, _buf: &[u8], _offset: usize) -> Result<usize, SvsmError> {
        Err(SvsmError::FileSystem(FsError::not_supported()))
    }

    /// Total number of bytes represented by this buffer.
    ///
    /// # Returns
    ///
    /// Total number of bytes that can be copied from/to the buffer.
    fn size(&self) -> usize;
}

/// Struct to add a [`Buffer`] interface to a mutable `&[u8]` slice
#[derive(Debug)]
pub struct SliceMutRefBuffer<'a> {
    slice: &'a mut [u8],
}

impl<'a> SliceMutRefBuffer<'a> {
    pub fn new(slice: &'a mut [u8]) -> Self {
        Self { slice }
    }
}

impl Buffer for SliceMutRefBuffer<'_> {
    fn read_buffer(&self, buf: &mut [u8], offset: usize) -> Result<usize, SvsmError> {
        let size = cmp::min(buf.len(), self.slice.len() - offset);
        buf[..size].clone_from_slice(&self.slice[offset..offset + size]);
        Ok(size)
    }

    fn write_buffer(&mut self, buf: &[u8], offset: usize) -> Result<usize, SvsmError> {
        let size = cmp::min(buf.len(), self.slice.len() - offset);
        self.slice[offset..offset + size].clone_from_slice(&buf[..size]);
        Ok(size)
    }

    fn size(&self) -> usize {
        self.slice.len()
    }
}

#[derive(Debug)]
/// Struct to add a [`Buffer`] interface to a non-mutable `&[u8]` slice
pub struct SliceRefBuffer<'a> {
    slice: &'a [u8],
}

impl<'a> SliceRefBuffer<'a> {
    pub fn new(slice: &'a [u8]) -> Self {
        Self { slice }
    }
}

impl Buffer for SliceRefBuffer<'_> {
    fn read_buffer(&self, buf: &mut [u8], offset: usize) -> Result<usize, SvsmError> {
        let size = cmp::min(buf.len(), self.slice.len() - offset);
        buf[..size].clone_from_slice(&self.slice[offset..offset + size]);
        Ok(size)
    }

    fn size(&self) -> usize {
        self.slice.len()
    }
}

#[derive(Debug)]
pub struct UserBuffer {
    addr: VirtAddr,
    size: usize,
}

impl UserBuffer {
    pub fn new(addr: VirtAddr, size: usize) -> Self {
        Self { addr, size }
    }
}

impl Buffer for UserBuffer {
    fn read_buffer(&self, buf: &mut [u8], offset: usize) -> Result<usize, SvsmError> {
        let size = cmp::min(buf.len(), self.size.checked_sub(offset).unwrap());
        if size > 0 {
            copy_from_user(self.addr + offset, &mut buf[..size])?;
        }
        Ok(size)
    }

    fn write_buffer(&mut self, buf: &[u8], offset: usize) -> Result<usize, SvsmError> {
        let size = cmp::min(buf.len(), self.size.checked_sub(offset).unwrap());
        if size > 0 {
            copy_to_user(&buf[..size], self.addr + offset)?;
        }
        Ok(size)
    }

    fn size(&self) -> usize {
        self.size
    }
}

mod tests {
    use super::SliceMutRefBuffer;
    use super::SliceRefBuffer;
    use super::UserBuffer;
    use crate::address::VirtAddr;
    use crate::fs::Buffer;

    #[test]
    fn test_slice_ref_buffer_read_buffer() {
        let s = [0u8; 100];
        let sref = SliceRefBuffer::new(&s);

        let mut b = [0u8; 10];

        {
            let res = sref.read_buffer(&mut b, 0);
            assert_eq!(res.ok(), Some(b.len()));
        }

        {
            let res = sref.read_buffer(&mut b, 95);
            assert_eq!(res.ok(), Some(5));
        }
    }

    #[test]
    #[should_panic]
    fn test_slice_ref_buffer_read_buffer_invalid_offset_panics() {
        let s = [0u8; 100];
        let sref = SliceRefBuffer::new(&s);

        let mut b = [0u8; 10];

        let res = sref.read_buffer(&mut b, 120);
        assert_eq!(res.ok(), Some(0));
    }

    #[test]
    fn test_slice_ref_buffer_size() {
        let s = [0u8; 100];
        let sref = SliceRefBuffer::new(&s);
        assert_eq!(sref.size(), s.len());
    }

    #[test]
    fn test_slice_mut_ref_buffer_read_buffer() {
        let mut s = [0u8; 100];
        let sref = SliceMutRefBuffer::new(&mut s);

        let mut b = [0u8; 10];

        {
            let res = sref.read_buffer(&mut b, 0);
            assert_eq!(res.ok(), Some(b.len()));
        }

        {
            let res = sref.read_buffer(&mut b, 95);
            assert_eq!(res.ok(), Some(5));
        }
    }

    #[test]
    #[should_panic]
    fn test_slice_mut_ref_buffer_read_buffer_invalid_offset_panics() {
        let mut s = [0u8; 100];
        let sref = SliceMutRefBuffer::new(&mut s);

        let mut b = [0u8; 10];

        let _ = sref.read_buffer(&mut b, 120);
    }

    #[test]
    #[should_panic]
    fn test_slice_mut_ref_buffer_write_buffer_invalid_offset_panics() {
        let mut s = [0u8; 100];
        let mut sref = SliceMutRefBuffer::new(&mut s);

        let b = [0u8; 10];

        let _ = sref.write_buffer(&b, 120);
    }

    #[test]
    fn test_slice_mut_ref_buffer_size() {
        let mut s = [0u8; 100];
        let sref = SliceMutRefBuffer::new(&mut s);
        assert_eq!(sref.size(), s.len());
    }

    #[test]
    fn test_slice_mut_ref_buffer_write_buffer() {
        let b = [0u8, 1, 2, 3, 4, 5];

        {
            let mut s = [0u8; 100];
            let res = {
                let mut sref = SliceMutRefBuffer::new(&mut s);
                sref.write_buffer(&b, 0)
            };
            assert_eq!(res.ok(), Some(b.len()));
            assert_eq!(s[..b.len()], b);
        }

        {
            let mut s = [0u8; 100];
            let res = {
                let mut sref = SliceMutRefBuffer::new(&mut s);
                sref.write_buffer(&b, 95)
            };
            assert_eq!(res.ok(), Some(5));
            assert_eq!(s[95..], b[..5]);
        }
    }

    #[test]
    fn test_user_buffer_size() {
        const SIZE: usize = 100;
        let s = [0u8; SIZE];
        let test_address = VirtAddr::from(s.as_ptr());
        let user_buffer = UserBuffer::new(test_address, SIZE);
        assert_eq!(user_buffer.size(), SIZE);
    }

    #[test]
    fn test_user_buffer_read_buffer() {
        const SIZE: usize = 100;
        let s = [0u8; SIZE];
        let test_address = VirtAddr::from(s.as_ptr());
        let user_buffer = UserBuffer::new(test_address, SIZE);

        let mut b = [0u8; 10];

        {
            let res = user_buffer.read_buffer(&mut b, 0);
            assert_eq!(res.ok(), Some(b.len()));
        }

        {
            let res = user_buffer.read_buffer(&mut b, 95);
            assert_eq!(res.ok(), Some(5));
        }
    }
}

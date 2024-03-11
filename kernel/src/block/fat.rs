use super::BlockDev;
use core::fmt;

/// Adapter BlockDev <-> fatfs::{Read+Write+Seek}
#[derive(Debug)]
pub struct Wrap<'a, T: BlockDev> {
    dev: &'a mut T,
    offset: usize,
}

#[derive(Debug, PartialEq, Copy, Clone)]
pub enum WIoError {
    Intr,
    Unex,
    Wzero,
    General,
}

impl fatfs::IoError for WIoError {
    fn is_interrupted(&self) -> bool {
        *self == WIoError::Intr
    }

    fn new_unexpected_eof_error() -> Self {
        WIoError::Unex
    }

    fn new_write_zero_error() -> Self {
        WIoError::Wzero
    }
}

impl<'a, T: BlockDev> Wrap<'a, T> {
    pub fn new(dev: &'a mut T) -> Self {
        Wrap { dev, offset: 0 }
    }

    fn seek_from_start(&mut self, pos: usize) -> Result<u64, WIoError> {
        if pos < self.dev.size() {
            self.offset = pos;
            Ok(pos as u64)
        } else {
            Err(WIoError::General)
        }
    }
}

impl<'a, T: BlockDev> fatfs::Write for Wrap<'_, T> {
    fn write(&mut self, buf: &[u8]) -> Result<usize, Self::Error> {
        match self.dev.write(buf, self.offset) {
            Ok(n) => {
                self.offset += n;
                Ok(n)
            }
            Err(_) => Err(WIoError::General),
        }
    }

    fn flush(&mut self) -> Result<(), Self::Error> {
        Ok(())
    }
}

impl<'a, T: BlockDev> fatfs::IoBase for Wrap<'_, T> {
    type Error = WIoError;
}

impl<'a, T: BlockDev> fatfs::Read for Wrap<'_, T> {
    fn read(&mut self, buf: &mut [u8]) -> Result<usize, Self::Error> {
        match self.dev.read(buf, self.offset) {
            Ok(n) => {
                self.offset += n;
                Ok(n)
            }
            Err(_) => Err(WIoError::General),
        }
    }
}

impl<'a, T: BlockDev> fatfs::Seek for Wrap<'_, T> {
    fn seek(&mut self, pos: fatfs::SeekFrom) -> Result<u64, Self::Error> {
        match pos {
            fatfs::SeekFrom::Current(n) => self.seek_from_start(self.offset + n as usize),
            fatfs::SeekFrom::Start(n) => self.seek_from_start(n as usize),
            fatfs::SeekFrom::End(n) => self.seek_from_start(self.dev.size() - n as usize),
        }
    }
}

/// Wrapper around fatfs::FatType to implement fmt::Display on it.
/// Is there a better way to do this?
#[derive(Copy, Clone, Debug)]
pub struct FatType(pub fatfs::FatType);

impl fmt::Display for FatType {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self.0 {
            fatfs::FatType::Fat12 => f.write_str("Fat 12"),
            fatfs::FatType::Fat16 => f.write_str("Fat 16"),
            fatfs::FatType::Fat32 => f.write_str("Fat 32"),
        }
    }
}

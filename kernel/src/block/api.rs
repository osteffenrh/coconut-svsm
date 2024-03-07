// SPDX-License-Identifier: MIT OR Apache-2.0
//
// Copyright (c) 2024 Red Hat, Inc.
//
// Author: Oliver Steffen <osteffen@redhat.com>

use crate::error::SvsmError;

pub trait BlockDev {
    fn read(&self, buf: &mut [u8], offset: usize) -> Result<usize, SvsmError>;

    fn write(&mut self, buf: &[u8], offset: usize) -> Result<usize, SvsmError>;

    fn size(&self) -> usize;
}

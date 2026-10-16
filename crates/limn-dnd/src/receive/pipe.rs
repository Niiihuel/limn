//! The worker-owned bounded pipe shared by incoming text and URI lists.

use std::{os::fd::OwnedFd, sync::Arc, time::Instant};

use rustix::{
    fs::{fcntl_getfl, fcntl_setfl, OFlags},
    io::{read, Errno},
    pipe::{pipe_with, PipeFlags},
};

use super::OfferId;
use crate::ReceiveError;

pub(super) struct ReceivePipe {
    reader: Arc<OwnedFd>,
    bytes: Vec<u8>,
    max_bytes: usize,
    deadline: Instant,
    terminal: bool,
}

impl ReceivePipe {
    pub(super) fn new(
        _id: OfferId,
        max_bytes: usize,
        deadline: Instant,
    ) -> Result<(Self, OwnedFd), ReceiveError> {
        Self::bounded(max_bytes, deadline)
    }

    pub(super) fn bounded(
        max_bytes: usize,
        deadline: Instant,
    ) -> Result<(Self, OwnedFd), ReceiveError> {
        if !(1..=1024 * 1024).contains(&max_bytes) {
            return Err(ReceiveError::InvalidLimits);
        }
        let (reader, writer) = pipe_with(PipeFlags::CLOEXEC).map_err(|_| ReceiveError::Io)?;
        let flags = fcntl_getfl(&reader).map_err(|_| ReceiveError::Io)?;
        // A nonblocking writer can make the source fail after a partial transfer.
        fcntl_setfl(&reader, flags | OFlags::NONBLOCK).map_err(|_| ReceiveError::Io)?;
        Ok((
            Self {
                reader: Arc::new(reader),
                bytes: Vec::with_capacity(max_bytes + 1),
                max_bytes,
                deadline,
                terminal: false,
            },
            writer,
        ))
    }

    pub(super) fn readiness_fd(&self) -> Arc<OwnedFd> {
        self.reader.clone()
    }

    pub(super) fn read_available(&mut self, now: Instant) -> Result<Option<Vec<u8>>, ReceiveError> {
        if self.terminal {
            return Err(ReceiveError::InvalidPhase);
        }
        if now >= self.deadline {
            return self.fail(ReceiveError::Timeout);
        }
        let mut buffer = [0; 4096];
        let remaining = (self.max_bytes + 1 - self.bytes.len()).min(buffer.len());
        match read(&self.reader, &mut buffer[..remaining]) {
            Ok(0) => {
                self.terminal = true;
                Ok(Some(std::mem::take(&mut self.bytes)))
            }
            Ok(count) => {
                self.bytes.extend_from_slice(&buffer[..count]);
                if self.bytes.len() > self.max_bytes {
                    self.fail(ReceiveError::TooLarge)
                } else {
                    Ok(None)
                }
            }
            Err(Errno::AGAIN | Errno::INTR) => Ok(None),
            Err(_) => self.fail(ReceiveError::Io),
        }
    }

    fn fail(&mut self, error: ReceiveError) -> Result<Option<Vec<u8>>, ReceiveError> {
        self.terminal = true;
        self.bytes.clear();
        Err(error)
    }
}

#[cfg(test)]
#[path = "pipe_tests.rs"]
mod tests;

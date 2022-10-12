use crate::{Bufferable, ReadLayered, Status};
use std::io::{self, IoSliceMut, Read};

/// Adapts an `&[u8]` to implement [`ReadLayered`].
pub struct SliceReader<'slice> {
    slice: &'slice [u8],
}

impl<'slice> SliceReader<'slice> {
    /// Construct a new `SliceReader` which wraps `slice`.
    #[inline]
    pub fn new(slice: &'slice [u8]) -> Self {
        Self { slice }
    }
}

impl<'slice> ReadLayered for SliceReader<'slice> {
    #[inline]
    fn read_with_status(&mut self, buf: &mut [u8]) -> io::Result<(usize, Status)> {
        let size = Read::read(&mut self.slice, buf)?;
        Ok((
            size,
            if self.slice.is_empty() {
                Status::End
            } else {
                Status::active()
            },
        ))
    }

    #[inline]
    fn read_vectored_with_status(
        &mut self,
        bufs: &mut [IoSliceMut<'_>],
    ) -> io::Result<(usize, Status)> {
        let size = Read::read_vectored(&mut self.slice, bufs)?;
        Ok((
            size,
            if self.slice.is_empty() {
                Status::End
            } else {
                Status::active()
            },
        ))
    }
}

impl<'slice> Bufferable for SliceReader<'slice> {
    #[inline]
    fn abandon(&mut self) {
        self.slice = &[];
    }

    #[inline]
    fn suggested_buffer_size(&self) -> usize {
        // This is just writing values to memory, so no need to buffer.
        0
    }
}

impl<'slice> Read for SliceReader<'slice> {
    #[inline]
    fn read(&mut self, buf: &mut [u8]) -> io::Result<usize> {
        Read::read(&mut self.slice, buf)
    }

    #[inline]
    fn read_vectored(&mut self, bufs: &mut [io::IoSliceMut<'_>]) -> io::Result<usize> {
        Read::read_vectored(&mut self.slice, bufs)
    }

    #[cfg(can_vector)]
    #[inline]
    fn is_read_vectored(&self) -> bool {
        Read::is_read_vectored(&self.slice)
    }

    #[inline]
    fn read_to_end(&mut self, buf: &mut Vec<u8>) -> io::Result<usize> {
        Read::read_to_end(&mut self.slice, buf)
    }

    #[inline]
    fn read_to_string(&mut self, buf: &mut String) -> io::Result<usize> {
        Read::read_to_string(&mut self.slice, buf)
    }

    #[inline]
    fn read_exact(&mut self, buf: &mut [u8]) -> io::Result<()> {
        Read::read_exact(&mut self.slice, buf)
    }
}

#[test]
fn test_slice_read_with_status() {
    let mut reader = SliceReader::new(b"hello world!");
    let mut buf = vec![0; 5];
    assert_eq!(
        reader.read_with_status(&mut buf).unwrap(),
        (5, Status::active())
    );
    assert_eq!(buf, b"hello");
    assert_eq!(
        reader.read_with_status(&mut buf).unwrap(),
        (5, Status::active())
    );
    assert_eq!(buf, b" worl");
    assert_eq!(reader.read_with_status(&mut buf).unwrap(), (2, Status::End));
    assert_eq!(&buf[..2], b"d!");
    assert_eq!(reader.read_with_status(&mut buf).unwrap(), (0, Status::End));
}

#[test]
fn test_slice_read() {
    let mut reader = SliceReader::new(b"hello world!");
    let mut buf = vec![0; 5];
    assert_eq!(reader.read(&mut buf).unwrap(), 5);
    assert_eq!(buf, b"hello");
    assert_eq!(reader.read(&mut buf).unwrap(), 5);
    assert_eq!(buf, b" worl");
    assert_eq!(reader.read(&mut buf).unwrap(), 2);
    assert_eq!(&buf[..2], b"d!");
    assert_eq!(reader.read(&mut buf).unwrap(), 0);
}

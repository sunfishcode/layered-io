use super::{Bufferable, Status};
use std::io::{self, IoSliceMut, Read};

/// An extension of [`Read`], with `read_with_status` and
/// `read_vectored_with_status` which return status information and zero is not
/// special-cased. It also allows streams to specify a `minimum_buffer_size`.
pub trait ReadLayered: Read + Bufferable {
    /// Like [`Read::read`], but also returns a `Status`.
    fn read_with_status(&mut self, buf: &mut [u8]) -> io::Result<(usize, Status)>;

    /// Like [`Read::read_vectored`], but also returns a `Status`.
    #[inline]
    fn read_vectored_with_status(
        &mut self,
        bufs: &mut [IoSliceMut<'_>],
    ) -> io::Result<(usize, Status)> {
        default_read_vectored_with_status(self, bufs)
    }

    /// Like `Read::read_exact`, but uses `read_with_status` to avoid
    /// performing an extra `read` at the end.
    #[inline]
    fn read_exact_using_status(&mut self, buf: &mut [u8]) -> io::Result<Status> {
        default_read_exact_using_status(self, buf)
    }

    /// Some streams require a buffer of at least a certain size.
    #[inline]
    fn minimum_buffer_size(&self) -> usize {
        0
    }
}

/// Default implementation of [`Read::read`] in terms of
/// [`ReadLayered::read_with_status`].
#[inline]
pub fn default_read<Inner: ReadLayered + ?Sized>(
    inner: &mut Inner,
    buf: &mut [u8],
) -> io::Result<usize> {
    inner.read_with_status(buf).and_then(to_std_io_read_result)
}

/// Default implementation of [`Read::read_vectored`] in terms of
/// [`ReadLayered::read_vectored_with_status`].
pub fn default_read_vectored<Inner: ReadLayered + ?Sized>(
    inner: &mut Inner,
    bufs: &mut [IoSliceMut<'_>],
) -> io::Result<usize> {
    inner
        .read_vectored_with_status(bufs)
        .and_then(to_std_io_read_result)
}

/// Default implementation of [`Read::read_to_end`] in terms of
/// [`ReadLayered::read_with_status`].
#[allow(clippy::indexing_slicing)]
pub fn default_read_to_end<Inner: ReadLayered + ?Sized>(
    inner: &mut Inner,
    buf: &mut Vec<u8>,
) -> io::Result<usize> {
    let start_len = buf.len();
    let buffer_size = inner.suggested_buffer_size();
    let mut read_len = buffer_size;
    loop {
        let read_pos = buf.len();

        // Allocate space in the buffer. This needlessly zeros out the
        // memory, however the current way to avoid it is to be part of the
        // standard library so that we can make assumptions about the
        // compiler not exploiting undefined behavior.
        // https://github.com/rust-lang/rust/issues/42788 for details.
        buf.resize(read_pos + read_len, 0);

        match inner.read_with_status(&mut buf[read_pos..]) {
            Ok((size, status)) => {
                buf.resize(read_pos + size, 0);
                match status {
                    Status::Open(_) => {
                        read_len -= size;
                        if read_len < inner.minimum_buffer_size() {
                            read_len += buffer_size;
                        }
                    }
                    Status::End => return Ok(buf.len() - start_len),
                }
            }
            Err(ref e) if e.kind() == io::ErrorKind::Interrupted => {}
            Err(e) => {
                buf.resize(start_len, 0);
                return Err(e);
            }
        }
    }
}

/// Default implementation of [`Read::read_to_string`] in terms of
/// [`Read::read_to_end`].
pub fn default_read_to_string<Inner: ReadLayered + ?Sized>(
    inner: &mut Inner,
    buf: &mut String,
) -> io::Result<usize> {
    // Allocate a `Vec` and read into it. This needlessly allocates,
    // rather than reading directly into `buf`'s buffer, but similarly
    // avoids issues of undefined behavior for now.
    let mut vec = Vec::new();
    let size = inner.read_to_end(&mut vec)?;
    let new = String::from_utf8(vec).map_err(|e| io::Error::new(io::ErrorKind::Other, e))?;
    buf.push_str(&new);
    Ok(size)
}

/// Default implementation of [`ReadLayered::read_exact_using_status`] in terms of
/// [`ReadLayered::read_with_status`].
#[allow(clippy::indexing_slicing)]
pub fn default_read_exact_using_status<Inner: ReadLayered + ?Sized>(
    inner: &mut Inner,
    mut buf: &mut [u8],
) -> io::Result<Status> {
    let mut result_status = Status::active();

    while !buf.is_empty() {
        match inner.read_with_status(buf) {
            Ok((size, status)) => {
                let t = buf;
                buf = &mut t[size..];
                if status.is_end() {
                    result_status = status;
                    break;
                }
            }
            Err(e) => return Err(e),
        }
    }

    if buf.is_empty() {
        Ok(result_status)
    } else {
        Err(io::Error::new(
            io::ErrorKind::UnexpectedEof,
            "failed to fill whole buffer",
        ))
    }
}

/// Default implementation of [`ReadLayered::read_vectored_with_status`] in
/// terms of [`ReadLayered::read_with_status`].
pub fn default_read_vectored_with_status<Inner: ReadLayered + ?Sized>(
    inner: &mut Inner,
    bufs: &mut [IoSliceMut<'_>],
) -> io::Result<(usize, Status)> {
    let buf = bufs
        .iter_mut()
        .find(|b| !b.is_empty())
        .map_or(&mut [][..], |b| &mut **b);
    inner.read_with_status(buf)
}

/// Default implementation of [`Read::is_read_vectored`] accompanying
/// [`default_read_vectored_with_status`].
#[cfg(can_vector)]
pub fn default_is_read_vectored<Inner: ReadLayered + ?Sized>(_inner: &Inner) -> bool {
    false
}

/// Translate from `read_with_status`'s return value with independent size and
/// status to a [`std::io::Read::read`] return value where 0 is special-cased
/// to mean end-of-stream, an `io::ErrorKind::Interrupted` error is used to
/// indicate a zero-length read, and pushes are not reported.
pub fn to_std_io_read_result(size_and_status: (usize, Status)) -> io::Result<usize> {
    match size_and_status {
        (0, Status::Open(_)) => Err(io::Error::new(
            io::ErrorKind::Interrupted,
            "read zero bytes from stream",
        )),
        (size, _) => Ok(size),
    }
}

impl<R: ReadLayered> ReadLayered for Box<R> {
    #[inline]
    fn read_with_status(&mut self, buf: &mut [u8]) -> io::Result<(usize, Status)> {
        self.as_mut().read_with_status(buf)
    }

    #[inline]
    fn read_vectored_with_status(
        &mut self,
        bufs: &mut [IoSliceMut<'_>],
    ) -> io::Result<(usize, Status)> {
        self.as_mut().read_vectored_with_status(bufs)
    }

    #[inline]
    fn minimum_buffer_size(&self) -> usize {
        self.as_ref().minimum_buffer_size()
    }
}

impl<R: ReadLayered> ReadLayered for &mut R {
    #[inline]
    fn read_with_status(&mut self, buf: &mut [u8]) -> io::Result<(usize, Status)> {
        (**self).read_with_status(buf)
    }

    #[inline]
    fn read_vectored_with_status(
        &mut self,
        bufs: &mut [IoSliceMut<'_>],
    ) -> io::Result<(usize, Status)> {
        (**self).read_vectored_with_status(bufs)
    }

    #[inline]
    fn minimum_buffer_size(&self) -> usize {
        (**self).minimum_buffer_size()
    }
}

use crate::{Activity, Bufferable, Status};
use std::io::{self, IoSlice, Write};

/// An extension of [`std::io::Write`], but adds a `close` function to allow
/// the stream to be closed and any outstanding errors to be reported, without
/// requiring a `sync_all`.
pub trait WriteLayered: Write + Bufferable {
    /// Flush any buffers and declare the end of the stream. Subsequent writes
    /// will fail.
    fn close(&mut self) -> io::Result<()>;

    /// Like [`Write::flush`], but has a status parameter describing
    /// the future of the stream:
    ///  - `Status::Ok(Activity::Active)`: do nothing
    ///  - `Status::Ok(Activity::Push)`: flush any buffers and transmit all
    ///    data
    ///  - `Status::End`: flush any buffers and declare the end of the stream
    ///
    /// Passing `Status::Ok(Activity::Push)` makes this behave the same as
    /// `flush()`.
    fn flush_with_status(&mut self, status: Status) -> io::Result<()> {
        match status {
            Status::Open(Activity::Active) => Ok(()),
            Status::Open(Activity::Push) => self.flush(),
            Status::End => self.close(),
        }
    }
}

/// Default implementation of [`Write::write_vectored`], in terms of
/// [`Write::write`].
pub fn default_write_vectored<Inner: Write + ?Sized>(
    inner: &mut Inner,
    bufs: &[IoSlice<'_>],
) -> io::Result<usize> {
    let buf = bufs
        .iter()
        .find(|b| !b.is_empty())
        .map_or(&[][..], |b| &**b);
    inner.write(buf)
}

/// Default implementation of [`Write::is_write_vectored`] accompanying
/// [`default_write_vectored`].
#[cfg(can_vector)]
#[inline]
pub fn default_is_write_vectored<Inner: Write + ?Sized>(_inner: &Inner) -> bool {
    false
}

/// Default implementation of [`Write::write_all`], in terms of
/// [`Write::write`].
#[allow(clippy::indexing_slicing)]
pub fn default_write_all<Inner: Write + ?Sized>(
    inner: &mut Inner,
    mut buf: &[u8],
) -> io::Result<()> {
    while !buf.is_empty() {
        match inner.write(buf) {
            Ok(0) => {
                return Err(io::Error::new(
                    io::ErrorKind::WriteZero,
                    "failed to write whole buffer",
                ));
            }
            Ok(n) => buf = &buf[n..],
            Err(ref e) if e.kind() == io::ErrorKind::Interrupted => {}
            Err(e) => return Err(e),
        }
    }
    Ok(())
}

/// Default implementation of [`Write::write_all_vectored`], in terms of
/// [`Write::write_vectored`].
#[cfg(write_all_vectored)]
pub fn default_write_all_vectored<Inner: Write + ?Sized>(
    inner: &mut Inner,
    mut bufs: &mut [IoSlice],
) -> io::Result<()> {
    // TODO: Use [rust-lang/rust#70436]once it stabilizes.
    // [rust-lang/rust#70436]: https://github.com/rust-lang/rust/issues/70436
    while !bufs.is_empty() {
        match inner.write_vectored(bufs) {
            Ok(nwritten) => bufs = advance(bufs, nwritten),
            Err(ref e) if e.kind() == io::ErrorKind::Interrupted => (),
            Err(e) => return Err(e),
        }
    }
    Ok(())
}

/// This will be obviated by [rust-lang/rust#62726].
///
/// [rust-lang/rust#62726]: https://github.com/rust-lang/rust/issues/62726.
///
/// Once this is removed, layered-io can become a `#![forbid(unsafe_code)]`
/// crate.
#[cfg(write_all_vectored)]
fn advance<'a, 'b>(bufs: &'b mut [IoSlice<'a>], n: usize) -> &'b mut [IoSlice<'a>] {
    use std::slice;

    // Number of buffers to remove.
    let mut remove = 0;
    // Total length of all the to be removed buffers.
    let mut accumulated_len = 0;
    for buf in bufs.iter() {
        if accumulated_len + buf.len() > n {
            break;
        }
        accumulated_len += buf.len();
        remove += 1;
    }

    #[allow(clippy::indexing_slicing)]
    let bufs = &mut bufs[remove..];
    if let Some(first) = bufs.first_mut() {
        let advance_by = n - accumulated_len;
        let mut ptr = first.as_ptr();
        let mut len = first.len();
        unsafe {
            ptr = ptr.add(advance_by);
            len -= advance_by;
            *first = IoSlice::<'a>::new(slice::from_raw_parts::<'a>(ptr, len));
        }
    }
    bufs
}

impl WriteLayered for std::io::Cursor<Vec<u8>> {
    #[inline]
    fn close(&mut self) -> io::Result<()> {
        self.set_position(self.get_ref().len().try_into().unwrap());
        Ok(())
    }
}

impl WriteLayered for std::io::Cursor<Box<[u8]>> {
    #[inline]
    fn close(&mut self) -> io::Result<()> {
        self.set_position(self.get_ref().len().try_into().unwrap());
        Ok(())
    }
}

impl WriteLayered for std::io::Cursor<&mut Vec<u8>> {
    #[inline]
    fn close(&mut self) -> io::Result<()> {
        self.set_position(self.get_ref().len().try_into().unwrap());
        Ok(())
    }
}

impl WriteLayered for std::io::Cursor<&mut [u8]> {
    #[inline]
    fn close(&mut self) -> io::Result<()> {
        self.set_position(self.get_ref().len().try_into().unwrap());
        Ok(())
    }
}

impl<W: WriteLayered> WriteLayered for Box<W> {
    #[inline]
    fn close(&mut self) -> io::Result<()> {
        self.as_mut().close()
    }
}

impl<W: WriteLayered> WriteLayered for &mut W {
    #[inline]
    fn close(&mut self) -> io::Result<()> {
        (**self).close()
    }
}

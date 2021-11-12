use crate::{Bufferable, WriteLayered};
use std::fmt::{self, Arguments};
use std::io::{self, IoSlice, Write};
#[cfg(windows)]
use io_extras::os::windows::{
    AsHandleOrSocket, AsRawHandleOrSocket, BorrowedHandleOrSocket, RawHandleOrSocket,
};
#[cfg(not(windows))]
use {
    io_lifetimes::{AsFd, BorrowedFd},
    io_extras::os::rustix::{AsRawFd, RawFd},
};

/// Adapts a [`std::io::Write`] to implement [`WriteLayered`].
pub struct LayeredWriter<Inner> {
    inner: Option<Inner>,
}

impl<Inner: Write> LayeredWriter<Inner> {
    /// Construct a new `LayeredWriter` which wraps `inner`.
    pub fn new(inner: Inner) -> Self {
        Self { inner: Some(inner) }
    }

    /// Gets a reference to the underlying writer.
    pub fn get_ref(&self) -> &Inner {
        self.inner
            .as_ref()
            .expect("get_ref() called on closed LayeredWriter")
    }

    /// Gets a mutable reference to the underlying writer.
    ///
    /// It is inadvisable to directly write to the underlying writer.
    pub fn get_mut(&mut self) -> &mut Inner {
        self.inner
            .as_mut()
            .expect("get_mut() called on closed LayeredWriter")
    }

    /// Close this `LayeredWriter` and return the inner stream.
    pub fn close_into_inner(mut self) -> io::Result<Inner> {
        match &mut self.inner {
            Some(_) => {
                let mut inner = self.inner.take().unwrap();
                inner.flush()?;
                Ok(inner)
            }
            None => Err(stream_already_ended()),
        }
    }

    /// Consume this `LayeredWriter` and return the inner stream.
    pub fn abandon_into_inner(mut self) -> Option<Inner> {
        self.inner.take()
    }
}

impl<Inner: Write> WriteLayered for LayeredWriter<Inner> {
    #[inline]
    fn close(&mut self) -> io::Result<()> {
        match &mut self.inner {
            Some(_) => self.inner.take().unwrap().flush(),
            None => Err(stream_already_ended()),
        }
    }
}

impl<Inner> Bufferable for LayeredWriter<Inner> {
    #[inline]
    fn abandon(&mut self) {
        self.inner = None;
    }
}

impl<Inner: Write> Write for LayeredWriter<Inner> {
    #[inline]
    fn write(&mut self, buf: &[u8]) -> io::Result<usize> {
        match &mut self.inner {
            Some(inner) => inner.write(buf).map_err(|e| {
                drop(self.inner.take().unwrap());
                e
            }),
            None => Err(stream_already_ended()),
        }
    }

    #[inline]
    fn flush(&mut self) -> io::Result<()> {
        match &mut self.inner {
            Some(inner) => inner.flush().map_err(|e| {
                drop(self.inner.take().unwrap());
                e
            }),
            None => Err(stream_already_ended()),
        }
    }

    #[inline]
    fn write_vectored(&mut self, bufs: &[IoSlice<'_>]) -> io::Result<usize> {
        match &mut self.inner {
            Some(inner) => inner.write_vectored(bufs).map_err(|e| {
                drop(self.inner.take().unwrap());
                e
            }),
            None => Err(stream_already_ended()),
        }
    }

    #[cfg(can_vector)]
    #[inline]
    fn is_write_vectored(&self) -> bool {
        match &self.inner {
            Some(inner) => inner.is_write_vectored(),
            None => false,
        }
    }

    #[inline]
    fn write_all(&mut self, buf: &[u8]) -> io::Result<()> {
        match &mut self.inner {
            Some(inner) => inner.write_all(buf).map_err(|e| {
                drop(self.inner.take().unwrap());
                e
            }),
            None => Err(stream_already_ended()),
        }
    }

    #[cfg(write_all_vectored)]
    #[inline]
    fn write_all_vectored(&mut self, bufs: &mut [IoSlice<'_>]) -> io::Result<()> {
        match &mut self.inner {
            Some(inner) => inner.write_all_vectored(bufs).map_err(|e| {
                drop(self.inner.take().unwrap());
                e
            }),
            None => Err(stream_already_ended()),
        }
    }

    #[inline]
    fn write_fmt(&mut self, fmt: Arguments<'_>) -> io::Result<()> {
        match &mut self.inner {
            Some(inner) => inner.write_fmt(fmt).map_err(|e| {
                drop(self.inner.take().unwrap());
                e
            }),
            None => Err(stream_already_ended()),
        }
    }
}

#[cfg(feature = "terminal-io")]
impl<RW: terminal_io::WriteTerminal> terminal_io::Terminal for LayeredWriter<RW> {}

#[cfg(feature = "terminal-io")]
impl<RW: terminal_io::WriteTerminal> terminal_io::WriteTerminal for LayeredWriter<RW> {
    #[inline]
    fn color_support(&self) -> terminal_io::TerminalColorSupport {
        self.inner.as_ref().unwrap().color_support()
    }

    #[inline]
    fn color_preference(&self) -> bool {
        self.inner.as_ref().unwrap().color_preference()
    }

    #[inline]
    fn is_output_terminal(&self) -> bool {
        self.inner
            .as_ref()
            .map(|c| c.is_output_terminal())
            .unwrap_or(false)
    }
}

#[cfg(not(windows))]
impl<Inner: Write + AsRawFd> AsRawFd for LayeredWriter<Inner> {
    #[inline]
    fn as_raw_fd(&self) -> RawFd {
        match &self.inner {
            Some(inner) => inner.as_raw_fd(),
            None => panic!("as_raw_fd() called on closed LayeredWriter"),
        }
    }
}

#[cfg(not(windows))]
impl<Inner: Write + AsFd> AsFd for LayeredWriter<Inner> {
    #[inline]
    fn as_fd(&self) -> BorrowedFd<'_> {
        match &self.inner {
            Some(inner) => inner.as_fd(),
            None => panic!("as_fd() called on closed LayeredWriter"),
        }
    }
}

#[cfg(windows)]
impl<Inner: Write + AsRawHandleOrSocket> AsRawHandleOrSocket for LayeredWriter<Inner> {
    #[inline]
    fn as_raw_handle_or_socket(&self) -> RawHandleOrSocket {
        match &self.inner {
            Some(inner) => inner.as_raw_handle_or_socket(),
            None => panic!("as_raw_handle_or_socket() called on closed LayeredWriter"),
        }
    }
}

#[cfg(windows)]
impl<Inner: Write + AsHandleOrSocket> AsHandleOrSocket for LayeredWriter<Inner> {
    #[inline]
    fn as_handle_or_socket(&self) -> BorrowedHandleOrSocket<'_> {
        match &self.inner {
            Some(inner) => inner.as_handle_or_socket(),
            None => panic!("as_handle_or_socket() called on closed LayeredWriter"),
        }
    }
}

impl<Inner: fmt::Debug> fmt::Debug for LayeredWriter<Inner> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let mut b = f.debug_struct("LayeredWriter");
        b.field("inner", &self.inner);
        b.finish()
    }
}

fn stream_already_ended() -> io::Error {
    io::Error::new(io::ErrorKind::BrokenPipe, "stream has already ended")
}

impl<Inner> Drop for LayeredWriter<Inner> {
    fn drop(&mut self) {
        assert!(self.inner.is_none(), "stream was not closed or abandoned");
    }
}

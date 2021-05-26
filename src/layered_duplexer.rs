use crate::{
    default_read, default_read_exact_using_status, default_read_to_end, default_read_to_string,
    default_read_vectored, Bufferable, ReadLayered, Status, WriteLayered,
};
use duplex::Duplex;
use std::{
    fmt::{self, Arguments},
    io::{self, IoSlice, IoSliceMut, Read, Write},
};
#[cfg(feature = "terminal-io")]
use terminal_io::DuplexTerminal;
#[cfg(not(windows))]
use unsafe_io::os::posish::{AsRawReadWriteFd, RawFd};
#[cfg(windows)]
use unsafe_io::os::windows::{AsRawReadWriteHandleOrSocket, RawHandleOrSocket};
use unsafe_io::OwnsRaw;

/// Adapts an `Read` + `Write` to implement `DuplexLayered`.
pub struct LayeredDuplexer<Inner> {
    inner: Option<Inner>,
    eos_as_push: bool,
    line_by_line: bool,
}

#[cfg(feature = "terminal-io")]
impl<Inner: DuplexTerminal> LayeredDuplexer<Inner> {
    /// Construct a new `LayeredDuplexer` which wraps `inner`, which implements
    /// `AsUnsafeHandle`, and automatically sets the `line_by_line` setting if
    /// appropriate.
    pub fn maybe_terminal(inner: Inner) -> Self {
        let line_by_line = inner.is_line_by_line();

        if line_by_line {
            Self::line_by_line(inner)
        } else {
            Self::new(inner)
        }
    }
}

impl<Inner: Read + Write> LayeredDuplexer<Inner> {
    /// Construct a new `LayeredDuplexer` which wraps `inner` with default
    /// settings.
    pub fn new(inner: Inner) -> Self {
        Self {
            inner: Some(inner),
            eos_as_push: false,
            line_by_line: false,
        }
    }

    /// Construct a new `LayeredDuplexer` which wraps `inner`. When `inner`
    /// reports end of stream (by returning 0), report a push but keep the
    /// stream open and continue to read data on it.
    ///
    /// For example, when reading a file, when the reader reaches the end of
    /// the file it will report it, but consumers may wish to continue reading
    /// in case additional data is appended to the file.
    pub fn with_eos_as_push(inner: Inner) -> Self {
        Self {
            inner: Some(inner),
            eos_as_push: true,
            line_by_line: false,
        }
    }

    /// Construct a new `LayeredDuplexer` which wraps an `inner` which reads
    /// its input line-by-line, such as stdin on a terminal.
    pub fn line_by_line(inner: Inner) -> Self {
        Self {
            inner: Some(inner),
            eos_as_push: false,
            line_by_line: true,
        }
    }

    /// Close this `LayeredDuplexer` and return the inner stream.
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

    /// Consume this `LayeredDuplexer` and return the inner stream.
    pub fn abandon_into_inner(mut self) -> Option<Inner> {
        self.inner.take()
    }
}

impl<Inner: Read + Write> ReadLayered for LayeredDuplexer<Inner> {
    #[inline]
    fn read_with_status(&mut self, buf: &mut [u8]) -> io::Result<(usize, Status)> {
        if self.inner.is_none() {
            return Ok((0, Status::End));
        }
        match self.inner.as_mut().unwrap().read(buf) {
            Ok(0) if !buf.is_empty() => {
                if self.eos_as_push {
                    Ok((0, Status::push()))
                } else {
                    drop(self.inner.take().unwrap());
                    Ok((0, Status::End))
                }
            }
            Ok(size) => {
                if self.line_by_line && buf[size - 1] == b'\n' {
                    Ok((size, Status::push()))
                } else {
                    Ok((size, Status::active()))
                }
            }
            Err(ref e) if e.kind() == io::ErrorKind::Interrupted => Ok((0, Status::active())),
            Err(e) => {
                self.abandon();
                Err(e)
            }
        }
    }

    #[inline]
    fn read_vectored_with_status(
        &mut self,
        bufs: &mut [IoSliceMut<'_>],
    ) -> io::Result<(usize, Status)> {
        if self.inner.is_none() {
            return Ok((0, Status::End));
        }
        match self.inner.as_mut().unwrap().read_vectored(bufs) {
            Ok(0) if !bufs.iter().all(|b| b.is_empty()) => {
                if self.eos_as_push {
                    Ok((0, Status::push()))
                } else {
                    drop(self.inner.take().unwrap());
                    Ok((0, Status::End))
                }
            }
            Ok(size) => {
                if self.line_by_line {
                    let mut i = size;
                    let mut saw_line = false;
                    for buf in bufs.iter() {
                        if i < buf.len() {
                            saw_line = buf[i - 1] == b'\n';
                            break;
                        }
                        i -= bufs.len();
                    }
                    if saw_line {
                        return Ok((size, Status::push()));
                    }
                }

                Ok((size, Status::active()))
            }
            Err(ref e) if e.kind() == io::ErrorKind::Interrupted => Ok((0, Status::active())),
            Err(e) => {
                self.abandon();
                Err(e)
            }
        }
    }
}

impl<Inner: Read + Write> Read for LayeredDuplexer<Inner> {
    #[inline]
    fn read(&mut self, buf: &mut [u8]) -> io::Result<usize> {
        default_read(self, buf)
    }

    #[inline]
    fn read_vectored(&mut self, bufs: &mut [IoSliceMut<'_>]) -> io::Result<usize> {
        default_read_vectored(self, bufs)
    }

    #[cfg(can_vector)]
    #[inline]
    fn is_read_vectored(&self) -> bool {
        match &self.inner {
            Some(inner) => inner.is_read_vectored(),
            None => false,
        }
    }

    #[inline]
    fn read_to_end(&mut self, buf: &mut Vec<u8>) -> io::Result<usize> {
        default_read_to_end(self, buf)
    }

    #[inline]
    fn read_to_string(&mut self, buf: &mut String) -> io::Result<usize> {
        default_read_to_string(self, buf)
    }

    #[inline]
    fn read_exact(&mut self, buf: &mut [u8]) -> io::Result<()> {
        default_read_exact_using_status(self, buf)?;
        Ok(())
    }
}

impl<Inner: Read + Write> WriteLayered for LayeredDuplexer<Inner> {
    #[inline]
    fn close(&mut self) -> io::Result<()> {
        match &mut self.inner {
            Some(_) => self.inner.take().unwrap().flush(),
            None => Err(stream_already_ended()),
        }
    }
}

impl<Inner: Read + Write> Write for LayeredDuplexer<Inner> {
    #[inline]
    fn write(&mut self, buf: &[u8]) -> io::Result<usize> {
        match &mut self.inner {
            Some(inner) => inner.write(buf),
            None => Err(stream_already_ended()),
        }
    }

    #[inline]
    fn flush(&mut self) -> io::Result<()> {
        match &mut self.inner {
            Some(inner) => inner.flush(),
            None => Err(stream_already_ended()),
        }
    }

    #[inline]
    fn write_vectored(&mut self, bufs: &[IoSlice<'_>]) -> io::Result<usize> {
        match &mut self.inner {
            Some(inner) => inner.write_vectored(bufs),
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
            Some(inner) => inner.write_all(buf),
            None => Err(stream_already_ended()),
        }
    }

    #[cfg(write_all_vectored)]
    #[inline]
    fn write_all_vectored(&mut self, bufs: &mut [IoSlice<'_>]) -> io::Result<()> {
        match &mut self.inner {
            Some(inner) => inner.write_all_vectored(bufs),
            None => Err(stream_already_ended()),
        }
    }

    #[inline]
    fn write_fmt(&mut self, fmt: Arguments<'_>) -> io::Result<()> {
        match &mut self.inner {
            Some(inner) => inner.write_fmt(fmt),
            None => Err(stream_already_ended()),
        }
    }
}

impl<Inner> Bufferable for LayeredDuplexer<Inner> {
    #[inline]
    fn abandon(&mut self) {
        self.inner = None;
    }
}

impl<Inner: Read + Write + Duplex> Duplex for LayeredDuplexer<Inner> {}

#[cfg(feature = "terminal-io")]
impl<RW: terminal_io::DuplexTerminal> terminal_io::Terminal for LayeredDuplexer<RW> {}

#[cfg(feature = "terminal-io")]
impl<RW: terminal_io::DuplexTerminal> terminal_io::ReadTerminal for LayeredDuplexer<RW> {
    #[inline]
    fn is_line_by_line(&self) -> bool {
        self.inner
            .as_ref()
            .map(|c| c.is_line_by_line())
            .unwrap_or(false)
    }

    #[inline]
    fn is_input_terminal(&self) -> bool {
        self.inner
            .as_ref()
            .map(|c| c.is_input_terminal())
            .unwrap_or(false)
    }
}

#[cfg(feature = "terminal-io")]
impl<RW: terminal_io::DuplexTerminal> terminal_io::WriteTerminal for LayeredDuplexer<RW> {
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

#[cfg(feature = "terminal-io")]
impl<RW: terminal_io::DuplexTerminal> terminal_io::DuplexTerminal for LayeredDuplexer<RW> {}

#[cfg(not(windows))]
impl<Inner: Duplex + AsRawReadWriteFd> AsRawReadWriteFd for LayeredDuplexer<Inner> {
    #[inline]
    fn as_raw_read_fd(&self) -> RawFd {
        match &self.inner {
            Some(inner) => inner.as_raw_read_fd(),
            None => panic!("as_raw_read_fd() called on closed LayeredDuplexer"),
        }
    }

    #[inline]
    fn as_raw_write_fd(&self) -> RawFd {
        match &self.inner {
            Some(inner) => inner.as_raw_write_fd(),
            None => panic!("as_raw_write_fd() called on closed LayeredDuplexer"),
        }
    }
}

#[cfg(windows)]
impl<Inner: Duplex + AsRawReadWriteHandleOrSocket> AsRawReadWriteHandleOrSocket
    for LayeredDuplexer<Inner>
{
    #[inline]
    fn as_raw_read_handle_or_socket(&self) -> RawHandleOrSocket {
        match &self.inner {
            Some(inner) => inner.as_raw_read_handle_or_socket(),
            None => panic!("as_raw_read_handle_or_socket() called on closed LayeredDuplexer"),
        }
    }

    #[inline]
    fn as_raw_write_handle_or_socket(&self) -> RawHandleOrSocket {
        match &self.inner {
            Some(inner) => inner.as_raw_write_handle_or_socket(),
            None => panic!("as_raw_write_handle_or_socket() called on closed LayeredDuplexer"),
        }
    }
}

// Safety: `LayeredDuplexer` implements `OwnsRaw` if `Inner` does.
unsafe impl<Inner: Duplex + OwnsRaw> OwnsRaw for LayeredDuplexer<Inner> {}

impl<Inner: fmt::Debug> fmt::Debug for LayeredDuplexer<Inner> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let mut b = f.debug_struct("LayeredDuplexer");
        b.field("inner", &self.inner);
        b.finish()
    }
}

fn stream_already_ended() -> io::Error {
    io::Error::new(io::ErrorKind::BrokenPipe, "stream has already ended")
}

impl<Inner> Drop for LayeredDuplexer<Inner> {
    fn drop(&mut self) {
        assert!(self.inner.is_none(), "stream was not closed or abandoned");
    }
}

#[test]
fn test_layered_duplexion() {
    let mut input = io::Cursor::new(b"hello world".to_vec());
    let mut reader = LayeredDuplexer::new(&mut input);
    let mut s = String::new();
    reader.read_to_string(&mut s).unwrap();
    assert_eq!(s, "hello world");
}

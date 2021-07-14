use crate::{
    default_read, default_read_exact_using_status, default_read_to_end, default_read_to_string,
    default_read_vectored, Bufferable, ReadLayered, Status,
};
#[cfg(not(windows))]
use io_lifetimes::{AsFd, BorrowedFd};
use std::{
    fmt,
    io::{self, IoSliceMut, Read},
};
#[cfg(feature = "terminal-io")]
use terminal_io::ReadTerminal;
#[cfg(windows)]
use unsafe_io::os::windows::{AsHandleOrSocket, BorrowedHandleOrSocket};

/// Adapts an `Read` to implement `ReadLayered`.
pub struct LayeredReader<Inner> {
    inner: Option<Inner>,
    eos_as_push: bool,
    line_by_line: bool,
}

#[cfg(feature = "terminal-io")]
impl<Inner: ReadTerminal> LayeredReader<Inner> {
    /// Construct a new `LayeredReader` which wraps `inner`, which implements
    /// `ReadTerminal`, and automatically sets the `line_by_line` setting if
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

impl<Inner: Read> LayeredReader<Inner> {
    /// Construct a new `LayeredReader` which wraps `inner` with default
    /// settings.
    pub fn new(inner: Inner) -> Self {
        Self {
            inner: Some(inner),
            eos_as_push: false,
            line_by_line: false,
        }
    }

    /// Construct a new `LayeredReader` which wraps `inner`. When `inner`
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

    /// Construct a new `LayeredReader` which wraps an `inner` which reads its
    /// input line-by-line, such as stdin on a terminal.
    pub fn line_by_line(inner: Inner) -> Self {
        Self {
            inner: Some(inner),
            eos_as_push: false,
            line_by_line: true,
        }
    }

    /// Consume this `LayeredReader` and return the inner stream.
    pub fn abandon_into_inner(self) -> Option<Inner> {
        self.inner
    }
}

impl<Inner: Read> ReadLayered for LayeredReader<Inner> {
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

impl<Inner> Bufferable for LayeredReader<Inner> {
    #[inline]
    fn abandon(&mut self) {
        self.inner = None;
    }
}

impl<Inner: Read> Read for LayeredReader<Inner> {
    #[inline]
    fn read(&mut self, buf: &mut [u8]) -> io::Result<usize> {
        default_read(self, buf).map_err(|e| {
            drop(self.inner.take().unwrap());
            e
        })
    }

    #[inline]
    fn read_vectored(&mut self, bufs: &mut [IoSliceMut<'_>]) -> io::Result<usize> {
        default_read_vectored(self, bufs).map_err(|e| {
            drop(self.inner.take().unwrap());
            e
        })
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
        default_read_to_end(self, buf).map_err(|e| {
            drop(self.inner.take().unwrap());
            e
        })
    }

    #[inline]
    fn read_to_string(&mut self, buf: &mut String) -> io::Result<usize> {
        default_read_to_string(self, buf).map_err(|e| {
            drop(self.inner.take().unwrap());
            e
        })
    }

    #[inline]
    fn read_exact(&mut self, buf: &mut [u8]) -> io::Result<()> {
        default_read_exact_using_status(self, buf)
            .map(|_status| ())
            .map_err(|e| {
                drop(self.inner.take().unwrap());
                e
            })
    }
}

#[cfg(feature = "terminal-io")]
impl<RW: Read + terminal_io::Terminal> terminal_io::Terminal for LayeredReader<RW> {}

#[cfg(feature = "terminal-io")]
impl<RW: terminal_io::ReadTerminal> terminal_io::ReadTerminal for LayeredReader<RW> {
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

#[cfg(not(windows))]
impl<Inner: Read + AsFd> AsFd for LayeredReader<Inner> {
    #[inline]
    fn as_fd(&self) -> BorrowedFd<'_> {
        match &self.inner {
            Some(inner) => inner.as_fd(),
            None => panic!("as_fd() called on closed LayeredReader"),
        }
    }
}

#[cfg(windows)]
impl<Inner: Read + AsHandleOrSocket> AsHandleOrSocket for LayeredReader<Inner> {
    #[inline]
    fn as_handle_or_socket(&self) -> BorrowedHandleOrSocket<'_> {
        match &self.inner {
            Some(inner) => inner.as_handle_or_socket(),
            None => panic!("as_handle_or_socket() called on closed LayeredReader"),
        }
    }
}

impl<Inner: fmt::Debug> fmt::Debug for LayeredReader<Inner> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let mut b = f.debug_struct("LayeredReader");
        b.field("inner", &self.inner);
        b.finish()
    }
}

#[test]
fn test_layered_reader() {
    let mut input = io::Cursor::new(b"hello world");
    let mut reader = LayeredReader::new(&mut input);
    let mut s = String::new();
    reader.read_to_string(&mut s).unwrap();
    assert_eq!(s, "hello world");
}

use crate::{Activity, Bufferable, Status};
use futures_io::AsyncWrite;
use std::io::{self, IoSlice};
use std::ops::DerefMut;
use std::pin::Pin;
use std::task::{Context, Poll};

/// An extension of [`std::io::Write`], but adds a `close` function to allow
/// the stream to be closed and any outstanding errors to be reported, without
/// requiring a `sync_all`.
pub trait AsyncWriteLayered: AsyncWrite + Bufferable {
    /// Like [`Write::flush`], but has a status parameter describing
    /// the future of the stream:
    ///  - `Status::Ok(Activity::Active)`: do nothing
    ///  - `Status::Ok(Activity::Push)`: flush any buffers and transmit all
    ///    data
    ///  - `Status::End`: flush any buffers and declare the end of the stream
    ///
    /// Passing `Status::Ok(Activity::Push)` makes this behave the same as
    /// `flush()`.
    ///
    /// [`Write::flush`]: std::io::Write::flush
    fn flush_with_status(
        self: Pin<&mut Self>,
        cx: &mut Context<'_>,
        status: Status,
    ) -> Poll<io::Result<()>> {
        match status {
            Status::Open(Activity::Active) => Poll::Ready(Ok(())),
            Status::Open(Activity::Push) => AsyncWrite::poll_flush(self, cx),
            Status::End => AsyncWrite::poll_close(self, cx),
        }
    }
}

/// Default implementation of [`AsyncWrite::poll_write_vectored`], in terms of
/// [`AsyncWrite::poll_write`].
pub fn default_poll_write_vectored<Inner: AsyncWrite + ?Sized>(
    inner: Pin<&mut Inner>,
    cx: &mut Context<'_>,
    bufs: &[IoSlice<'_>],
) -> Poll<io::Result<usize>> {
    let buf = bufs
        .iter()
        .find(|b| !b.is_empty())
        .map_or(&[][..], |b| &**b);
    AsyncWrite::poll_write(inner, cx, buf)
}

impl<W: AsyncWriteLayered + Unpin> AsyncWriteLayered for Box<W> {}

impl<W: AsyncWriteLayered + Unpin> AsyncWriteLayered for &mut W {}

impl<P> AsyncWriteLayered for Pin<P>
where
    P: DerefMut + Unpin,
    P::Target: AsyncWriteLayered + Unpin,
{
}

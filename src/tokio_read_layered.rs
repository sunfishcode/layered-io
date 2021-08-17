use super::{Bufferable, Status};
use std::io;
use std::pin::Pin;
use std::task::{Context, Poll};
use tokio::io::{AsyncRead, ReadBuf};

/// An extension of [`AsyncRead`], with `read_with_status` and
/// `read_vectored_with_status` which return status information and zero is not
/// special-cased. It also allows streams to specify a `minimum_buffer_size`.
pub trait TokioReadLayered: AsyncRead + Bufferable {
    /// Like [`AsyncRead::poll_read`], but also returns a `Status`.
    fn poll_read_with_status(
        self: Pin<&mut Self>,
        cx: &mut Context<'_>,
        buf: &mut ReadBuf,
    ) -> Poll<io::Result<((), Status)>>;

    /// Some streams require a buffer of at least a certain size.
    #[inline]
    fn minimum_buffer_size(&self) -> usize {
        0
    }
}

/// Default implementation of [`AsyncRead::poll_read`] in terms of
/// [`AsyncReadLayered::poll_read_with_status`].
#[inline]
pub fn tokio_default_poll_read<Inner: TokioReadLayered + ?Sized>(
    inner: Pin<&mut Inner>,
    cx: &mut Context<'_>,
    buf: &mut ReadBuf,
) -> Poll<io::Result<()>> {
    let start_len = buf.filled().len();
    inner
        .poll_read_with_status(cx, buf)
        .map(|result| result.and_then(|((), status)| to_tokio_read_result(start_len, status, buf)))
}

impl<R: TokioReadLayered + Unpin> TokioReadLayered for Box<R> {
    #[inline]
    fn poll_read_with_status(
        mut self: Pin<&mut Self>,
        cx: &mut Context<'_>,
        buf: &mut ReadBuf,
    ) -> Poll<io::Result<((), Status)>> {
        self.as_mut().poll_read_with_status(cx, buf)
    }

    #[inline]
    fn minimum_buffer_size(&self) -> usize {
        self.as_ref().minimum_buffer_size()
    }
}

impl<R: TokioReadLayered + Unpin> TokioReadLayered for &mut R {
    #[inline]
    fn poll_read_with_status(
        mut self: Pin<&mut Self>,
        cx: &mut Context<'_>,
        buf: &mut ReadBuf,
    ) -> Poll<io::Result<((), Status)>> {
        TokioReadLayered::poll_read_with_status(Pin::new(*self), cx, buf)
    }

    #[inline]
    fn minimum_buffer_size(&self) -> usize {
        (**self).minimum_buffer_size()
    }
}

/// Translate from `read_with_status`'s return value with independent size and
/// status to a [`std::io::Read::read`] return value where 0 is special-cased
/// to mean end-of-stream, an `io::ErrorKind::Interrupted` error is used to
/// indicate a zero-length read, and pushes are not reported.
fn to_tokio_read_result(start_len: usize, status: Status, buf: &ReadBuf) -> io::Result<()> {
    let size = buf.filled().len() - start_len;
    match (size, status) {
        (0, Status::Open(_)) => Err(io::Error::new(
            io::ErrorKind::Interrupted,
            "read zero bytes from stream",
        )),
        (_, _) => Ok(()),
    }
}

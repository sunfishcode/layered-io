use super::{Bufferable, Status};
use crate::to_std_io_read_result;
use futures_io::AsyncRead;
use std::{
    io::{self, IoSliceMut},
    pin::Pin,
    task::{Context, Poll},
};

/// An extension of [`AsyncRead`], with `read_with_status` and
/// `read_vectored_with_status` which return status information and zero is not
/// special-cased. It also allows streams to specify a `minimum_buffer_size`.
pub trait AsyncReadLayered: AsyncRead + Bufferable {
    /// Like [`AsyncRead::poll_read`], but also returns a `Status`.
    fn poll_read_with_status(
        self: Pin<&mut Self>,
        cx: &mut Context<'_>,
        buf: &mut [u8],
    ) -> Poll<io::Result<(usize, Status)>>;

    /// Like [`AsyncRead::poll_read_vectored`], but also returns a `Status`.
    #[inline]
    fn poll_read_vectored_with_status(
        self: Pin<&mut Self>,
        cx: &mut Context<'_>,
        bufs: &mut [IoSliceMut<'_>],
    ) -> Poll<io::Result<(usize, Status)>> {
        default_poll_read_vectored_with_status(self, cx, bufs)
    }

    /// Some streams require a buffer of at least a certain size.
    #[inline]
    fn minimum_buffer_size(&self) -> usize {
        0
    }
}

/// Default implementation of [`AsyncRead::poll_read`] in terms of
/// [`AsyncReadLayered::poll_read_with_status`].
#[inline]
pub fn default_poll_read<Inner: AsyncReadLayered + ?Sized>(
    inner: Pin<&mut Inner>,
    cx: &mut Context<'_>,
    buf: &mut [u8],
) -> Poll<io::Result<usize>> {
    inner
        .poll_read_with_status(cx, buf)
        .map(|result| result.and_then(to_std_io_read_result))
}

/// Default implementation of [`AsyncRead::poll_read_vectored`] in terms of
/// [`AsyncReadLayered::poll_read_vectored_with_status`].
pub fn default_poll_read_vectored<Inner: AsyncReadLayered + ?Sized>(
    inner: Pin<&mut Inner>,
    cx: &mut Context<'_>,
    bufs: &mut [IoSliceMut<'_>],
) -> Poll<io::Result<usize>> {
    inner
        .poll_read_vectored_with_status(cx, bufs)
        .map(|result| result.and_then(to_std_io_read_result))
}

/// Default implementation of
/// [`AsyncReadLayered::poll_read_vectored_with_status`] in terms of
/// [`AsyncReadLayered::poll_read_with_status`].
pub fn default_poll_read_vectored_with_status<Inner: AsyncReadLayered + ?Sized>(
    inner: Pin<&mut Inner>,
    cx: &mut Context<'_>,
    bufs: &mut [IoSliceMut<'_>],
) -> Poll<io::Result<(usize, Status)>> {
    let buf = bufs
        .iter_mut()
        .find(|b| !b.is_empty())
        .map_or(&mut [][..], |b| &mut **b);
    inner.poll_read_with_status(cx, buf)
}

impl<R: AsyncReadLayered + Unpin> AsyncReadLayered for Box<R> {
    #[inline]
    fn poll_read_with_status(
        mut self: Pin<&mut Self>,
        cx: &mut Context<'_>,
        buf: &mut [u8],
    ) -> Poll<io::Result<(usize, Status)>> {
        self.as_mut().poll_read_with_status(cx, buf)
    }

    #[inline]
    fn poll_read_vectored_with_status(
        mut self: Pin<&mut Self>,
        cx: &mut Context<'_>,
        bufs: &mut [IoSliceMut<'_>],
    ) -> Poll<io::Result<(usize, Status)>> {
        self.as_mut().poll_read_vectored_with_status(cx, bufs)
    }

    #[inline]
    fn minimum_buffer_size(&self) -> usize {
        self.as_ref().minimum_buffer_size()
    }
}

impl<R: AsyncReadLayered + Unpin> AsyncReadLayered for &mut R {
    #[inline]
    fn poll_read_with_status(
        mut self: Pin<&mut Self>,
        cx: &mut Context<'_>,
        buf: &mut [u8],
    ) -> Poll<io::Result<(usize, Status)>> {
        AsyncReadLayered::poll_read_with_status(Pin::new(*self), cx, buf)
    }

    #[inline]
    fn poll_read_vectored_with_status(
        mut self: Pin<&mut Self>,
        cx: &mut Context<'_>,
        bufs: &mut [IoSliceMut<'_>],
    ) -> Poll<io::Result<(usize, Status)>> {
        AsyncReadLayered::poll_read_vectored_with_status(Pin::new(&mut **self), cx, bufs)
    }

    #[inline]
    fn minimum_buffer_size(&self) -> usize {
        (**self).minimum_buffer_size()
    }
}

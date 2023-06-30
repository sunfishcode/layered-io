use std::ops::DerefMut;
use std::pin::Pin;

/// A trait to help with buffering on top of `ReadLayered` and `WriteLayered`.
pub trait Bufferable {
    /// Close the resource and abandon any pending buffered contents or errors.
    fn abandon(&mut self);

    /// A suggested size, in bytes, for buffering for performance.
    #[inline]
    fn suggested_buffer_size(&self) -> usize {
        default_suggested_buffer_size(self)
    }
}

/// Default implementation of `Bufferable::abandon`, which does nothing.
#[inline]
pub fn default_suggested_buffer_size<Inner: Bufferable + ?Sized>(_inner: &Inner) -> usize {
    // At the time of this writing, this is the same as `DEFAULT_BUF_SIZE`
    // in libstd.
    0x2000
}

impl<B: Bufferable> Bufferable for Box<B> {
    #[inline]
    fn abandon(&mut self) {
        self.as_mut().abandon()
    }

    #[inline]
    fn suggested_buffer_size(&self) -> usize {
        self.as_ref().suggested_buffer_size()
    }
}

impl<B: Bufferable> Bufferable for &mut B {
    #[inline]
    fn abandon(&mut self) {
        (**self).abandon()
    }

    #[inline]
    fn suggested_buffer_size(&self) -> usize {
        (**self).suggested_buffer_size()
    }
}

impl Bufferable for std::io::Cursor<Vec<u8>> {
    #[inline]
    fn abandon(&mut self) {
        self.set_position(self.get_ref().len().try_into().unwrap())
    }

    #[inline]
    fn suggested_buffer_size(&self) -> usize {
        0
    }
}

impl Bufferable for std::io::Cursor<Box<[u8]>> {
    #[inline]
    fn abandon(&mut self) {
        self.set_position(self.get_ref().len().try_into().unwrap())
    }

    #[inline]
    fn suggested_buffer_size(&self) -> usize {
        0
    }
}

impl Bufferable for std::io::Cursor<&mut Vec<u8>> {
    #[inline]
    fn abandon(&mut self) {
        self.set_position(self.get_ref().len().try_into().unwrap())
    }

    #[inline]
    fn suggested_buffer_size(&self) -> usize {
        0
    }
}

impl Bufferable for std::io::Cursor<&mut [u8]> {
    #[inline]
    fn abandon(&mut self) {
        self.set_position(self.get_ref().len().try_into().unwrap())
    }

    #[inline]
    fn suggested_buffer_size(&self) -> usize {
        0
    }
}

impl<P> Bufferable for Pin<P>
where
    P: DerefMut + Unpin,
    P::Target: Bufferable + Unpin,
{
    #[inline]
    fn abandon(&mut self) {
        Pin::get_mut(self.as_mut()).abandon()
    }

    #[inline]
    fn suggested_buffer_size(&self) -> usize {
        self.as_ref().suggested_buffer_size()
    }
}

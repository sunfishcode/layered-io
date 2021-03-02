use std::{convert::TryInto, ops::DerefMut, pin::Pin};

/// A trait for output streams which can be *closed*, meaning subsequent writes
/// will fail instead of being transmitted.
pub trait Closeable {
    /// Flush any buffers and declare the end of the stream, reporting any
    /// errors that occur.
    fn close(&mut self) -> io::Result<()>;

    /// Close the resource and abandon any pending buffered contents or errors.
    fn abandon(&mut self);
}

impl<B: Closeable> Closeable for Box<B> {
    #[inline]
    fn close(&mut self) -> io::Result<()> {
        self.as_mut().close()
    }

    #[inline]
    fn abandon(&mut self) {
        self.as_mut().abandon()
    }
}

impl<B: Closeable> Closeable for &mut B {
    #[inline]
    fn close(&mut self) -> io::Result<()> {
        (**self).close()
    }

    #[inline]
    fn abandon(&mut self) {
        (**self).abandon()
    }
}

impl Closeable for std::io::Cursor<Vec<u8>> {
    #[inline]
    fn close(&mut self) -> io::Result<()> {
        // There will never be any errors to report.
        Ok(self.abandon())
    }

    #[inline]
    fn abandon(&mut self) {
        self.set_position(self.get_ref().len().try_into().unwrap())
    }
}

impl Closeable for std::io::Cursor<Box<[u8]>> {
    #[inline]
    fn close(&mut self) -> io::Result<()> {
        // There will never be any errors to report.
        Ok(self.abandon())
    }

    #[inline]
    fn abandon(&mut self) {
        self.set_position(self.get_ref().len().try_into().unwrap())
    }
}

impl Closeable for std::io::Cursor<&mut Vec<u8>> {
    #[inline]
    fn close(&mut self) -> io::Result<()> {
        // There will never be any errors to report.
        Ok(self.abandon())
    }

    #[inline]
    fn abandon(&mut self) {
        self.set_position(self.get_ref().len().try_into().unwrap())
    }
}

impl Closeable for std::io::Cursor<&mut [u8]> {
    #[inline]
    fn close(&mut self) -> io::Result<()> {
        // There will never be any errors to report.
        Ok(self.abandon())
    }

    #[inline]
    fn abandon(&mut self) {
        self.set_position(self.get_ref().len().try_into().unwrap())
    }
}

impl<P> Closeable for Pin<P>
where
    P: DerefMut + Unpin,
    P::Target: Closeable,
{
    #[inline]
    fn close(&mut self) -> io::Result<()> {
        self.as_mut().close()
    }

    #[inline]
    fn abandon(&mut self) {
        self.as_mut().abandon()
    }
}

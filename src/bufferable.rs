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

/// What is known about a stream in the future.
#[derive(Copy, Clone, Eq, PartialEq, Debug)]
pub enum Status {
    /// The stream is open.
    Open(Activity),

    /// The stream has ended. No more bytes will be transmitted.
    End,
}

impl Status {
    /// Return `Status::Open` with activity state `Active`.
    #[inline]
    pub fn active() -> Self {
        Self::Open(Activity::Active)
    }

    /// Return `Status::Open` with activity state `Push`.
    #[inline]
    pub fn push() -> Self {
        Self::Open(Activity::Push)
    }

    /// Shorthand for testing equality with `Status::End`.
    #[inline]
    pub fn is_end(self) -> bool {
        self == Self::End
    }

    /// Shorthand for testing equality with `Status::Open(Activity::Push)`.
    #[inline]
    pub fn is_push(self) -> bool {
        self == Self::Open(Activity::Push)
    }
}

/// For interactivity, it's desirable to avoid buffering data which is complete
/// enough to be actionable. `Activity` allows writers to notify the API at
/// points when the data provided is actionable and buffers should be flushed
/// to the reader.
///
/// Users that aren't implementing buffering can ignore this.
#[derive(Copy, Clone, Eq, PartialEq, Debug)]
pub enum Activity {
    /// The writer is actively writing and data may be buffered.
    Active,

    /// The writer has finished writing something actionable and requested
    /// buffering layers flush at this point.
    ///
    /// This is similar to the [`PSH` flag] in TCP.
    ///
    /// [`PSH` flag]: https://en.wikipedia.org/wiki/Transmission_Control_Protocol#TCP_segment_structure
    Push,
}

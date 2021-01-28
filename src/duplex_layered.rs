use crate::{ReadLayered, WriteLayered};
use duplex::{Duplex, HalfDuplex};

/// A trait which simply combines [`ReadLayered`], [`WriteLayered`], and
/// [`HalfDuplex`].
pub trait HalfDuplexLayered: HalfDuplex + ReadLayered + WriteLayered {}

impl<T: Duplex + ReadLayered + WriteLayered> HalfDuplexLayered for T {}

// TODO: `AsyncReadLayered` and `AsyncWriteLayered`?

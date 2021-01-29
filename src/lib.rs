//! I/O traits extending [`std::io::Read`] and [`std::io::Write`].

#![deny(missing_docs)]
#![cfg_attr(can_vector, feature(can_vector))]
#![cfg_attr(write_all_vectored, feature(write_all_vectored))]
#![cfg_attr(target_os = "wasi", feature(wasi_ext))]

#[cfg(feature = "futures-io")]
mod async_read_layered;
#[cfg(feature = "futures-io")]
mod async_write_layered;
mod bufferable;
mod duplex_layered;
mod layered_duplexer;
mod layered_reader;
mod layered_writer;
mod read_layered;
mod slice_reader;
mod status;
mod write_layered;

#[cfg(feature = "futures-io")]
pub use async_read_layered::{default_poll_read, default_poll_read_vectored, AsyncReadLayered};
#[cfg(feature = "futures-io")]
pub use async_write_layered::{default_poll_write_vectored, AsyncWriteLayered};
pub use bufferable::{default_suggested_buffer_size, Bufferable};
pub use duplex_layered::HalfDuplexLayered;
pub use layered_duplexer::LayeredDuplexer;
pub use layered_reader::LayeredReader;
pub use layered_writer::LayeredWriter;
#[cfg(can_vector)]
pub use read_layered::default_is_read_vectored;
pub use read_layered::{
    default_read, default_read_exact_using_status, default_read_to_end, default_read_to_string,
    default_read_vectored, to_std_io_read_result, ReadLayered,
};
pub use slice_reader::SliceReader;
pub use status::{Activity, Status};
#[cfg(can_vector)]
pub use write_layered::default_is_write_vectored;
#[cfg(write_all_vectored)]
pub use write_layered::default_write_all_vectored;
pub use write_layered::{default_write_all, default_write_vectored, WriteLayered};

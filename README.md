<div align="center">
  <h1><code>layered-io</code></h1>

  <p>
    <strong>I/O traits extending Read and Write</strong>
  </p>

  <p>
    <a href="https://github.com/sunfishcode/layered-io/actions?query=workflow%3ACI"><img src="https://github.com/sunfishcode/layered-io/workflows/CI/badge.svg" alt="Github Actions CI Status" /></a>
    <a href="https://crates.io/crates/layered-io"><img src="https://img.shields.io/crates/v/layered-io.svg" alt="crates.io page" /></a>
    <a href="https://docs.rs/layered-io"><img src="https://docs.rs/layered-io/badge.svg" alt="docs.rs docs" /></a>
  </p>
</div>

This crate defines [`ReadLayered`] and [`WriteLayered`] traits which extend
[`std::io::Read`] and [`std::io::Write`] with additional functionality
useful for performing I/O through layers of buffering and translation.

And it defines [`LayeredReader`], [`LayeredWriter`], and [`LayeredInteractor`]
types which implement [`ReadLayered`], [`WriteLayered`], and both,
respectively, by wrapping implementations of [`std::io::Read`],
[`std::io::Write`], and both, respectively.

[`ReadLayered`]: https://docs.rs/layered-io/latest/layered_io/trait.ReadLayered.html
[`WriteLayered`]: https://docs.rs/layered-io/latest/layered_io/trait.WriteLayered.html
[`std::io::Read`]: https://doc.rust-lang.org/std/io/trait.Read.html
[`std::io::Write`]: https://doc.rust-lang.org/std/io/trait.Write.html
[`LayeredReader`]: https://docs.rs/layered-io/latest/layered_io/struct.LayeredReader.html
[`LayeredWriter`]: https://docs.rs/layered-io/latest/layered_io/struct.LayeredWriter.html
[`LayeredInteractor`]: https://docs.rs/layered-io/latest/layered_io/struct.LayeredInteractor.html

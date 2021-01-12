<p>
  <p align="center"><img src="https://raw.githubusercontent.com/lipanski/mockito/master/docs/logo-black.png"></p>
  <p align="center">
    <a href="https://docs.rs/mockito"><img src="https://docs.rs/mockito/badge.svg"></a>
    <a href="https://crates.io/crates/mockito"><img src="https://img.shields.io/crates/v/mockito.svg"></a>
    <img src="https://img.shields.io/badge/rust%20version-%3E%3D1.42.0-orange">
    <a href="https://crates.io/crates/mockito"><img src="https://img.shields.io/crates/d/mockito"></a>
    <a href="https://travis-ci.com/lipanski/mockito"><img src="https://travis-ci.com/lipanski/mockito.svg?branch=master"></a>
    <a href="https://ci.appveyor.com/project/lipanski/mockito"><img src="https://ci.appveyor.com/api/projects/status/github/lipanski/mockito?branch=master&svg=true"></a>
  </p>
  <p align="center"><em>HTTP mocking for Rust!</em></p>
</p>

Get it on [crates.io](https://crates.io/crates/mockito/).

Documentation is available at <https://docs.rs/mockito>.

Before upgrading, make sure to check out the [changelog](https://github.com/lipanski/mockito/releases).

## Contribution Guidelines

1. Check the existing issues and pull requests.
2. One commit is one feature - consider squashing.
3. Format code with `cargo fmt`.
4. :shipit:

## Development

### Tests

Run tests:

```sh
cargo test
```

...or run tests using a different toolchain:

```sh
rustup run --install 1.42.0 cargo test
```

...or run tests while disabling the default features (e.g. the colors):

```sh
cargo test --no-default-features
```

### Code style

Mockito uses [rustfmt](https://github.com/rust-lang/rustfmt) as a general code style.

Install `rustfmt`:

```sh
rustup component add rustfmt
```

Format code:

```sh
cargo fmt
```

Some editors might provide a plugin to format your Rust code automatically.

### Linter

Mockito uses [clippy](https://github.com/rust-lang/rust-clippy) and it should be run always on the minimum supported Rust version, in order to ensure backwards compatibility.

Install `clippy`:

```sh
rustup component add clippy-preview
```

Run the linter on the minimum supported Rust version:

```sh
rustup run --install 1.42.0 cargo clippy --lib --tests --all-features -- -D clippy::pedantic -D clippy::nursery
```

### Release

Release:

```sh
cargo publish
```

### Benchmarks

Install `rust nightly`:

```sh
rustup install nightly
```

Run benchmarks:

```sh
rustup run nightly cargo bench
```

---

Logo courtesy to [http://niastudio.net](http://niastudio.net) :ok_hand:

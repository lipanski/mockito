![Mockito](http://lipanski.github.io/mockito/logo/logo-black.png)

[![Build Status](https://img.shields.io/crates/v/mockito.svg)](https://crates.io/crates/mockito) [![Build Status](https://travis-ci.org/lipanski/mockito.svg?branch=master)](https://travis-ci.org/lipanski/mockito)

HTTP mocking for Rust!

Get it on [crates.io](https://crates.io/crates/mockito/).

Documentation available [here](http://lipanski.github.io/mockito/).

Logo courtesy to [http://niastudio.net](http://niastudio.net).

## Development

Run tests:

```
cargo test
```

Generate docs:

```
rustdoc -o docs -L target/debug -L target/debug/deps --crate-name mockito src/lib.rs

# or

cargo doc --no-deps && cp -R target/doc/mockito/* docs/mockito
```

Release:

```
cargo package && cargo publish
```

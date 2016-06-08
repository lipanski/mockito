![Mockito](http://lipanski.github.io/mockito/logo-black.png)

HTTP mocking for Rust!

Get it on [crates.io](https://crates.io/crates/mockito/).

Documentation available [here](http://lipanski.github.io/mockito/).

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

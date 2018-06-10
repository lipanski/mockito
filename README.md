<p>
  <p align="center"><img src="http://lipanski.github.io/mockito/logo/logo-black.png"></p>
  <p align="center">
    <a href="https://crates.io/crates/mockito"><img src="https://img.shields.io/crates/v/mockito.svg"></a>
    <a href="https://docs.rs/mockito"><img src="https://docs.rs/mockito/badge.svg"></a>
    <a href="https://travis-ci.org/lipanski/mockito"><img src="https://travis-ci.org/lipanski/mockito.svg?branch=master"></a>
    <a href="https://ci.appveyor.com/project/lipanski/mockito"><img src="https://ci.appveyor.com/api/projects/status/github/lipanski/mockito?branch=master&svg=true"></a>
  </p>
  <p align="center"><em>HTTP mocking for Rust!</em></p>
</p>

Get it on [crates.io](https://crates.io/crates/mockito/).

Documentation available at <https://docs.rs/mockito>.

Before upgrading, make sure to check out the [changelog](https://github.com/lipanski/mockito/releases).

## Contribution Guidelines

1. Check the existing issues and pull requests.
2. One commit is one feature - consider squashing.
3. I'll handle versioning and generating docs, after the pull request gets merged.
4. :shipit:

## Development

Run tests:

```
cargo test --tests

# See https://github.com/lipanski/mockito/issues/36
cargo test --doc -- --test-threads=1
```

Generate docs:

```
rm -r target/doc/* && cargo doc --no-deps && rm -r docs/generated/* && cp -R target/doc/* docs/generated
```

Release:

```
cargo package && cargo publish
```

Run benchmarks:

```
rustup install nightly
rustup run nightly cargo bench
```

---

Logo courtesy to [http://niastudio.net](http://niastudio.net) :ok_hand:

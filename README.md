<p>
  <p align="center"><img src="http://lipanski.github.io/mockito/logo/logo-black.png"></p>
  <p align="center">
    <a href="https://crates.io/crates/mockito"><img src="https://img.shields.io/crates/v/mockito.svg"></a>
    <a href="https://travis-ci.org/lipanski/mockito"><img src="https://travis-ci.org/lipanski/mockito.svg?branch=master"></a>
  </p>
  <p align="center"><em>HTTP mocking for Rust!</em></p>
</p>

Get it on [crates.io](https://crates.io/crates/mockito/).

Documentation available at [http://lipanski.github.io/mockito/](http://lipanski.github.io/mockito/).

Before upgrading, make sure to check out the [changelog](https://github.com/lipanski/mockito/releases).

## Development

Run tests:

```
cargo test --no-fail-fast -- --test-threads=1
```

Generate docs:

```
rm -r target/doc/* && cargo doc --no-deps && rm -r docs/generated/* && cp -R target/doc/* docs/generated
```

Release:

```
cargo package && cargo publish
```

---

Logo courtesy to [http://niastudio.net](http://niastudio.net) :ok_hand:

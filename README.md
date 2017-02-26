<p>
  <p align="center">![Mockito](http://lipanski.github.io/mockito/logo/logo-black.png)</p>
  <p align="center">[![Build Status](https://img.shields.io/crates/v/mockito.svg)](https://crates.io/crates/mockito) [![Build Status](https://travis-ci.org/lipanski/mockito.svg?branch=master)](https://travis-ci.org/lipanski/mockito)</p>
</p>

HTTP mocking for Rust!

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

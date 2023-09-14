<p>
  <p align="center"><img src="https://raw.githubusercontent.com/lipanski/mockito/master/docs/logo-black.png"></p>
  <p align="center">
    <a href="https://docs.rs/mockito"><img src="https://docs.rs/mockito/badge.svg"></a>
    <a href="https://crates.io/crates/mockito"><img src="https://img.shields.io/crates/v/mockito.svg"></a>
    <img src="https://img.shields.io/badge/rust%20version-%3E%3D1.68.0-orange">
    <a href="https://crates.io/crates/mockito"><img src="https://img.shields.io/crates/d/mockito"></a>
    <a href="https://github.com/lipanski/mockito/actions/workflows/tests.yml/?branch=master"><img src="https://github.com/lipanski/mockito/actions/workflows/tests.yml/badge.svg?branch=master"></a>
  </p>
  <p align="center"><em>HTTP mocking for Rust!</em></p>
</p>

Mockito is a library for **generating and delivering HTTP mocks** in Rust. You can use it for integration testing 
or offline work. Mockito runs a local pool of HTTP servers which create, deliver and remove the mocks.

## Features

- Supports HTTP1/2
- Runs your tests in parallel
- Comes with a wide range of request matchers (Regex, JSON, query parameters etc.)
- Checks that a mock was called (spy)
- Mocks multiple hosts at the same time
- Exposes sync and async interfaces
- Prints out a colored diff of the last unmatched request in case of errors
- Simple, intuitive API
- An awesome logo


The full documentation is available at <https://docs.rs/mockito>.

Before upgrading, make sure to check out the [changelog](https://github.com/lipanski/mockito/releases).

## Getting Started

Add `mockito` to your `Cargo.toml` and start mocking:

```rust
#[test]
fn test_something() {
    // Request a new server from the pool
    let mut server = mockito::Server::new();

    // Use one of these addresses to configure your client
    let host = server.host_with_port();
    let url = server.url();

    // Create a mock
    let mock = server.mock("GET", "/hello")
      .with_status(201)
      .with_header("content-type", "text/plain")
      .with_header("x-api-key", "1234")
      .with_body("world")
      .create();

    // Any calls to GET /hello beyond this line will respond with 201, the
    // `content-type: text/plain` header and the body "world".

    // You can use `Mock::assert` to verify that your mock was called
    mock.assert();
}
```

If `Mock::assert` fails, a colored diff of the last unmatched request is displayed:

![colored-diff.png](https://raw.githubusercontent.com/lipanski/mockito/master/docs/colored-diff.png)

Use **matchers** to handle requests to the same endpoint in a different way:

```rust
#[test]
fn test_something() {
    let mut server = mockito::Server::new();

    server.mock("GET", "/greetings")
      .match_header("content-type", "application/json")
      .match_body(mockito::Matcher::PartialJsonString(
          "{\"greeting\": \"hello\"}".to_string(),
      ))
      .with_body("hello json")
      .create();

    server.mock("GET", "/greetings")
      .match_header("content-type", "application/text")
      .match_body(mockito::Matcher::Regex("greeting=hello".to_string()))
      .with_body("hello text")
      .create();
}
```

Start **multiple servers** to simulate requests to different hosts:

```rust
#[test]
fn test_something() {
    let mut twitter = mockito::Server::new();
    let mut github = mockito::Server::new();

    // These mocks will be available at `twitter.url()`
    let twitter_mock = twitter.mock("GET", "/api").create();

    // These mocks will be available at `github.url()`
    let github_mock = github.mock("GET", "/api").create();
}
```

Write **async** tests (make sure to use the `_async` methods!):

```rust
#[tokio::test]
async fn test_simple_route_mock_async() {
    let mut server = Server::new_async().await;
    let m1 = server.mock("GET", "/a").with_body("aaa").create_async().await;
    let m2 = server.mock("GET", "/b").with_body("bbb").create_async().await;

    let (m1, m2) = futures::join!(m1, m2);

    // You can use `Mock::assert_async` to verify that your mock was called
    // m1.assert_async().await;
    // m2.assert_async().await;
}
```

## Minimum supported Rust toolchain

The current minimum support Rust toolchain is **1.68.0**

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
rustup run --install 1.68.0 cargo test
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
rustup component add clippy
```

The linter is always run on the minimum supported Rust version:

```sh
rustup run --install 1.68.0 cargo clippy-mockito
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

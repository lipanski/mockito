![Mockito](http://lipanski.github.io/mockito/logo-medium.png)

Mocking web requests in Rust!

Works with [Hyper](http://hyper.rs/) and [TcpStream](https://doc.rust-lang.org/std/net/struct.TcpStream.html).

## Introduction

Mockito is a web server you can use to test HTTP requests in Rust without actually hitting the remote server.
It intercepts URLs wrapped within `mockito::url::Url` and responds with a preconfigured message. Currently, it can
only handle `&'str` URLs.

Mockito will match on the HTTP method and path:

```rust
mockito::mock("GET", "/hello?world=1")
```

But it can also match requests based on headers:

```rust
mockito::mock("POST", "/something").header("content-type", "application/json")
```

## Development

Run tests:

```
cargo test
```

Generate docs:

```
rustdoc -o docs -L target/debug -L target/debug/deps --crate-name mockito src/lib.rs
```

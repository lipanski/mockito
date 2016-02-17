# Mockito

**This is a work in progress.**

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

Mockito comes with three **feature flags**, all of them disabled by default:

- `use_hyper`: enable this if your crate uses hyper but you don't want to mock anything
- `mock_hyper` enable this if your crate uses hyper and you'd like Mockito to intercept and mock requests
- `mock_tcp_stream`: enable this if you'd like Mockito to intercept and mock requests from a `TcpStream`

## Mocking Hyper requests

Include Mockito in your `Cargo.toml`:

```rust
[dependencies.mockito]
version = "0.1.4"
features = ["use_hyper"]

[dev-dependencies.mockito]
version = "0.1.4"
features = ["mock_hyper"]
```

Wrap your Hyper URLs within `mockito::url::Url`:

```rust
use hyper::Client;
use mockito::url::Url;

fn greet() -> String {
  let url = Url("http://www.example.com/greet");

  let client = Client::new();
  let mut res = client.get(url).send().unwrap();

  let mut body = String::new();
  response.read_to_string(&mut body).unwrap();

  body
}
```

Write a test:

```rust
#[cfg(test)]
mod test {
  use mockito;
  use hyper::client::response::Response;

  #[test]
  fn test_with_mockito() {
    mockito::mock("GET", "/greet").respond_with("hello");

    assert_eq("hello".to_string(), greet());

    mockito::reset();
  }
}
```

## Mocking requests from a file

Mockito can respond to a request with the contents of a file.

Given the file `tests/files/response.http`:

```
HTTP/1.1 200 OK

hello
```

You can intercept a request to `GET /greet` with its contents by calling:

```rust
mockito::mock("GET", "/greet").respond_with_file("tests/files/response.http")
```

## Matching headers

```rust
mockito::mock("GET", "/greet")
  .header("user-agent", "rust")
  .header("authorization", "basic 1234")
  .respond_with("hello")
```

## Removing registered matchers

Every call to `respond_with` or `respond_with_file` will register a matcher on the Mockito server.

In some situations you might want to clean up the current matchers before running other tests.

You can do this by calling:

```rust
mockito::reset()
```

As Rust tests share some context, you might want to run this at the beginning/end of every test method.

## Enable Mockito only for some of your tests

You might want to run some tests against the Mockito server and others against the remote server - in order to validate
your implementation against a live system from time to time.

To achieve this, you'll need to remove the `dev-dependencies.mockito` entry from your `Cargo.toml` and
map the `mock_hyper` feature to an internal feature flag:

```
[features]
default = []
mocks = ["mockito/mock_hyper"]

[dependencies.mockito]
version = "0.1.4"
features = ["use_hyper"]
```

Write some tests:

```rust
#[cfg(test)]
#[cfg(feature = "mocks")]
mod test {
  use mockito;
  use hyper::client::response::Response;

  #[test]
  fn test_with_mockito() {
    mockito::mock("GET", "/greet").respond_with("hello");

    assert_eq!("hello", greet());

    mockito::reset();
  }
}

#[cfg(test)]
#[cfg(not(feature = "mocks"))]
mod test {
  #[test]
  #[should_panic]
  fn test_without_mockito() {
    assert_eq!("hello", greet());
  }
}
```

In order to run the Mockito-enabled tests, you'd call:

```sh
cargo test --features mocks
```

All your other tests can be run by calling the default:

```
cargo test
```

## Drawbacks

Mockito doesn't interpret the host inside your URLs. This means that `http://www.example.com/greet` and
`http://www.something.com/greet` will be handled the same way.

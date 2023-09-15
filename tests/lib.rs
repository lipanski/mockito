#[macro_use]
extern crate serde_json;

use mockito::{Matcher, Server};
use rand::distributions::Alphanumeric;
use rand::Rng;
use std::fmt::Display;
use std::fs;
use std::io::{BufRead, BufReader, Read, Write};
use std::net::TcpStream;
use std::path::Path;
use std::str::FromStr;
use std::sync::{Arc, Mutex};
use std::thread;

type Binary = Vec<u8>;

fn request_stream<S: Display, StrOrBytes: AsRef<[u8]>>(
    version: &str,
    host: S,
    route: &str,
    headers: &str,
    body: StrOrBytes,
) -> TcpStream {
    let host = host.to_string();
    let mut stream =
        TcpStream::connect(&host).unwrap_or_else(|_| panic!("couldn't connect to {}", &host));
    let mut message: Binary = Vec::new();
    for b in [route, " HTTP/", version, "\r\n", headers, "\r\n"]
        .join("")
        .as_bytes()
    {
        message.push(*b);
    }
    for b in body.as_ref().iter() {
        message.push(*b);
    }

    stream.write_all(&message).unwrap();

    stream
}

fn parse_stream(stream: TcpStream, skip_body: bool) -> (String, Vec<String>, Binary) {
    let mut reader = BufReader::new(stream);

    let mut status_line = String::new();
    reader.read_line(&mut status_line).unwrap();

    let mut headers = vec![];
    let mut content_length: Option<u64> = None;
    let mut is_chunked = false;
    loop {
        let mut header_line = String::new();
        reader.read_line(&mut header_line).unwrap();

        if header_line == "\r\n" {
            break;
        }

        if header_line.starts_with("transfer-encoding:") && header_line.contains("chunked") {
            is_chunked = true;
        }

        if header_line.starts_with("content-length:") {
            let mut parts = header_line.split(':');
            content_length = Some(u64::from_str(parts.nth(1).unwrap().trim()).unwrap());
        }

        headers.push(header_line.trim_end().to_string());
    }

    let mut body: Binary = Vec::new();
    if !skip_body {
        if let Some(content_length) = content_length {
            reader.take(content_length).read_to_end(&mut body).unwrap();
        } else if is_chunked {
            let mut chunk_size_buf = String::new();
            loop {
                chunk_size_buf.clear();
                reader.read_line(&mut chunk_size_buf).unwrap();

                let chunk_size = u64::from_str_radix(
                    chunk_size_buf.trim_matches(|c| c == '\r' || c == '\n'),
                    16,
                )
                .expect("chunk size");
                if chunk_size == 0 {
                    break;
                }

                (&mut reader)
                    .take(chunk_size)
                    .read_to_end(&mut body)
                    .unwrap();

                chunk_size_buf.clear();
                reader.read_line(&mut chunk_size_buf).unwrap();
            }
        }
    }

    (status_line, headers, body)
}

fn binary_request<S: Display, StrOrBytes: AsRef<[u8]>>(
    host: S,
    route: &str,
    headers: &str,
    body: StrOrBytes,
) -> (String, Vec<String>, Binary) {
    parse_stream(
        request_stream("1.1", host, route, headers, body),
        route.starts_with("HEAD"),
    )
}

fn request<S: Display>(host: S, route: &str, headers: &str) -> (String, Vec<String>, String) {
    let (status, headers, body) = binary_request(host, route, headers, "");
    let parsed_body: String = std::str::from_utf8(body.as_slice()).unwrap().to_string();
    (status, headers, parsed_body)
}

fn request_with_body<S: Display>(
    host: S,
    route: &str,
    headers: &str,
    body: &str,
) -> (String, Vec<String>, String) {
    let headers = format!("{}content-length: {}\r\n", headers, body.len());
    let (status, headers, body) = binary_request(host, route, &headers, body);
    let parsed_body: String = std::str::from_utf8(body.as_slice()).unwrap().to_string();
    (status, headers, parsed_body)
}

#[test]
fn test_create_starts_the_server() {
    let mut s = Server::new();
    s.mock("GET", "/").with_body("hello").create();

    let stream = TcpStream::connect(s.host_with_port());
    assert!(stream.is_ok());
}

#[test]
fn test_simple_route_mock() {
    let mut s = Server::new();
    s.mock("GET", "/hello").with_body("world").create();

    let (status_line, _, body) = request(&s.host_with_port(), "GET /hello", "");
    assert_eq!("HTTP/1.1 200 OK\r\n", status_line);
    assert_eq!("world", body);
}

#[test]
fn test_two_route_mocks() {
    let mut s = Server::new();
    s.mock("GET", "/a").with_body("aaa").create();
    s.mock("GET", "/b").with_body("bbb").create();

    let (_, _, body_a) = request(&s.host_with_port(), "GET /a", "");
    assert_eq!("aaa", body_a);

    let (_, _, body_b) = request(&s.host_with_port(), "GET /b", "");
    assert_eq!("bbb", body_b);
}

#[test]
fn test_no_match_returns_501() {
    let mut s = Server::new();
    s.mock("GET", "/").with_body("matched").create();

    let (status_line, _headers, body) = request(&s.host_with_port(), "GET /nope", "");
    assert_eq!("HTTP/1.1 501 Not Implemented\r\n", status_line);
    assert_eq!("", body);
}

#[test]
fn test_match_header() {
    let mut s = Server::new();
    s.mock("GET", "/")
        .match_header("content-type", "application/json")
        .with_body("{}")
        .create();

    s.mock("GET", "/")
        .match_header("content-type", "text/plain")
        .with_body("hello")
        .create();

    let (_, _, body_json) = request(
        &s.host_with_port(),
        "GET /",
        "content-type: application/json\r\n",
    );
    assert_eq!("{}", body_json);

    let (_, _, body_text) = request(&s.host_with_port(), "GET /", "content-type: text/plain\r\n");
    assert_eq!("hello", body_text);
}

#[test]
fn test_match_header_is_case_insensitive_on_the_field_name() {
    let mut s = Server::new();
    s.mock("GET", "/")
        .match_header("content-type", "text/plain")
        .create();

    let (uppercase_status_line, _, _) =
        request(&s.host_with_port(), "GET /", "Content-Type: text/plain\r\n");
    assert_eq!("HTTP/1.1 200 OK\r\n", uppercase_status_line);

    let (lowercase_status_line, _, _) =
        request(&s.host_with_port(), "GET /", "content-type: text/plain\r\n");
    assert_eq!("HTTP/1.1 200 OK\r\n", lowercase_status_line);
}

#[test]
fn test_match_multiple_headers() {
    let mut s = Server::new();
    s.mock("GET", "/")
        .match_header("Content-Type", "text/plain")
        .match_header("Authorization", "secret")
        .with_body("matched")
        .create();

    let (_, _, body_matching) = request(
        &s.host_with_port(),
        "GET /",
        "content-type: text/plain\r\nauthorization: secret\r\n",
    );
    assert_eq!("matched", body_matching);

    let (status_not_matching, _, _) = request(
        &s.host_with_port(),
        "GET /",
        "content-type: text/plain\r\nauthorization: meh\r\n",
    );
    assert_eq!("HTTP/1.1 501 Not Implemented\r\n", status_not_matching);
}

#[test]
fn test_match_header_any_matching() {
    let mut s = Server::new();
    s.mock("GET", "/")
        .match_header("Content-Type", Matcher::Any)
        .with_body("matched")
        .create();

    let (_, _, body) = request(&s.host_with_port(), "GET /", "content-type: something\r\n");
    assert_eq!("matched", body);
}

#[test]
fn test_match_header_any_not_matching() {
    let mut s = Server::new();
    s.mock("GET", "/")
        .match_header("Content-Type", Matcher::Any)
        .with_body("matched")
        .create();

    let (status, _, _) = request(&s.host_with_port(), "GET /", "");
    assert_eq!("HTTP/1.1 501 Not Implemented\r\n", status);
}

#[test]
fn test_match_header_missing_matching() {
    let mut s = Server::new();
    s.mock("GET", "/")
        .match_header("Authorization", Matcher::Missing)
        .create();

    let (status, _, _) = request(&s.host_with_port(), "GET /", "");
    assert_eq!("HTTP/1.1 200 OK\r\n", status);
}

#[test]
fn test_match_header_missing_not_matching() {
    let mut s = Server::new();
    s.mock("GET", "/")
        .match_header("Authorization", Matcher::Missing)
        .create();

    let (status, _, _) = request(&s.host_with_port(), "GET /", "Authorization: something\r\n");
    assert_eq!("HTTP/1.1 501 Not Implemented\r\n", status);
}

#[test]
fn test_match_header_missing_not_matching_even_when_empty() {
    let mut s = Server::new();
    s.mock("GET", "/")
        .match_header("Authorization", Matcher::Missing)
        .create();

    let (status, _, _) = request(&s.host_with_port(), "GET /", "Authorization:\r\n");
    assert_eq!("HTTP/1.1 501 Not Implemented\r\n", status);
}

#[test]
fn test_match_multiple_header_conditions_matching() {
    let mut s = Server::new();
    s.mock("GET", "/")
        .match_header("Hello", "World")
        .match_header("Content-Type", Matcher::Any)
        .match_header("Authorization", Matcher::Missing)
        .create();

    let (status, _, _) = request(
        &s.host_with_port(),
        "GET /",
        "Hello: World\r\nContent-Type: something\r\n",
    );
    assert_eq!("HTTP/1.1 200 OK\r\n", status);
}

#[test]
fn test_match_multiple_header_conditions_not_matching() {
    let mut s = Server::new();
    s.mock("GET", "/")
        .match_header("hello", "world")
        .match_header("Content-Type", Matcher::Any)
        .match_header("Authorization", Matcher::Missing)
        .create();

    let (status, _, _) = request(&s.host_with_port(), "GET /", "Hello: World\r\n");
    assert_eq!("HTTP/1.1 501 Not Implemented\r\n", status);
}

#[test]
fn test_match_any_body_by_default() {
    let mut s = Server::new();
    s.mock("POST", "/").create();

    let (status, _, _) = request_with_body(&s.host_with_port(), "POST /", "", "hello");
    assert_eq!("HTTP/1.1 200 OK\r\n", status);
}

#[test]
fn test_match_body() {
    let mut s = Server::new();
    s.mock("POST", "/").match_body("hello").create();

    let (status, _, _) = request_with_body(&s.host_with_port(), "POST /", "", "hello");
    assert_eq!("HTTP/1.1 200 OK\r\n", status);
}

#[test]
fn test_match_body_not_matching() {
    let mut s = Server::new();
    s.mock("POST", "/").match_body("hello").create();

    let (status, _, _) = request_with_body(&s.host_with_port(), "POST /", "", "bye");
    assert_eq!("HTTP/1.1 501 Not Implemented\r\n", status);
}

#[test]
fn test_match_binary_body() {
    let mut s = Server::new();
    s.mock("POST", "/")
        .match_body(Path::new("./tests/files/test_payload.bin"))
        .create();

    let mut file_content: Binary = Vec::new();
    fs::File::open("./tests/files/test_payload.bin")
        .unwrap()
        .read_to_end(&mut file_content)
        .unwrap();
    let content_length_header = format!("Content-Length: {}\r\n", file_content.len());
    let (status, _, _) = binary_request(
        &s.host_with_port(),
        "POST /",
        &content_length_header,
        file_content,
    );
    assert_eq!("HTTP/1.1 200 OK\r\n", status);
}

#[test]
fn test_does_not_match_binary_body() {
    let mut s = Server::new();
    s.mock("POST", "/")
        .match_body(Path::new("./tests/files/test_payload.bin"))
        .create();

    let file_content: Binary = (0..1024).map(|_| rand::random::<u8>()).collect();
    let content_length_header = format!("Content-Length: {}\r\n", file_content.len());
    let (status, _, _) = binary_request(
        &s.host_with_port(),
        "POST /",
        &content_length_header,
        file_content,
    );
    assert_eq!("HTTP/1.1 501 Not Implemented\r\n", status);
}

#[test]
fn test_match_body_with_regex() {
    let mut s = Server::new();
    s.mock("POST", "/")
        .match_body(Matcher::Regex("hello".to_string()))
        .create();

    let (status, _, _) = request_with_body(&s.host_with_port(), "POST /", "", "test hello test");
    assert_eq!("HTTP/1.1 200 OK\r\n", status);
}

#[test]
fn test_match_body_with_regex_not_matching() {
    let mut s = Server::new();
    s.mock("POST", "/")
        .match_body(Matcher::Regex("hello".to_string()))
        .create();

    let (status, _, _) = request_with_body(&s.host_with_port(), "POST /", "", "bye");
    assert_eq!("HTTP/1.1 501 Not Implemented\r\n", status);
}

#[test]
fn test_match_body_with_json() {
    let mut s = Server::new();
    s.mock("POST", "/")
        .match_body(Matcher::Json(json!({"hello":"world", "foo": "bar"})))
        .create();

    let (status, _, _) = request_with_body(
        &s.host_with_port(),
        "POST /",
        "",
        r#"{"hello":"world", "foo": "bar"}"#,
    );
    assert_eq!("HTTP/1.1 200 OK\r\n", status);
}

#[test]
fn test_match_body_with_more_headers_with_json() {
    let mut s = Server::new();
    s.mock("POST", "/")
        .match_body(Matcher::Json(json!({"hello":"world", "foo": "bar"})))
        .create();

    let headers = (0..15)
        .map(|n| {
            format!(
                "x-header-{}: foo-bar-value-zzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzz\r\n",
                n
            )
        })
        .collect::<Vec<String>>()
        .concat();

    let (status, _, _) = request_with_body(
        &s.host_with_port(),
        "POST /",
        &headers,
        r#"{"hello":"world", "foo": "bar"}"#,
    );
    assert_eq!("HTTP/1.1 200 OK\r\n", status);
}

#[test]
fn test_match_body_with_json_order() {
    let mut s = Server::new();
    s.mock("POST", "/")
        .match_body(Matcher::Json(json!({"foo": "bar", "hello": "world"})))
        .create();

    let (status, _, _) = request_with_body(
        &s.host_with_port(),
        "POST /",
        "",
        r#"{"hello":"world", "foo": "bar"}"#,
    );
    assert_eq!("HTTP/1.1 200 OK\r\n", status);
}

#[test]
fn test_match_body_with_json_string() {
    let mut s = Server::new();
    s.mock("POST", "/")
        .match_body(Matcher::JsonString(
            "{\"hello\":\"world\", \"foo\": \"bar\"}".to_string(),
        ))
        .create();

    let (status, _, _) = request_with_body(
        &s.host_with_port(),
        "POST /",
        "",
        r#"{"hello":"world", "foo": "bar"}"#,
    );
    assert_eq!("HTTP/1.1 200 OK\r\n", status);
}

#[test]
fn test_match_body_with_json_string_order() {
    let mut s = Server::new();
    s.mock("POST", "/")
        .match_body(Matcher::JsonString(
            "{\"foo\": \"bar\", \"hello\": \"world\"}".to_string(),
        ))
        .create();

    let (status, _, _) = request_with_body(
        &s.host_with_port(),
        "POST /",
        "",
        r#"{"hello":"world", "foo": "bar"}"#,
    );
    assert_eq!("HTTP/1.1 200 OK\r\n", status);
}

#[test]
fn test_match_body_with_partial_json() {
    let mut s = Server::new();
    s.mock("POST", "/")
        .match_body(Matcher::PartialJson(json!({"hello":"world"})))
        .create();

    let (status, _, _) = request_with_body(
        &s.host_with_port(),
        "POST /",
        "",
        r#"{"hello":"world", "foo": "bar"}"#,
    );
    assert_eq!("HTTP/1.1 200 OK\r\n", status);
}

#[test]
fn test_match_body_with_partial_json_and_extra_fields() {
    let mut s = Server::new();
    s.mock("POST", "/")
        .match_body(Matcher::PartialJson(json!({"hello":"world", "foo": "bar"})))
        .create();

    let (status, _, _) =
        request_with_body(&s.host_with_port(), "POST /", "", r#"{"hello":"world"}"#);
    assert_eq!("HTTP/1.1 501 Not Implemented\r\n", status);
}

#[test]
fn test_match_body_with_partial_json_string() {
    let mut s = Server::new();
    s.mock("POST", "/")
        .match_body(Matcher::PartialJsonString(
            "{\"hello\": \"world\"}".to_string(),
        ))
        .create();

    let (status, _, _) = request_with_body(
        &s.host_with_port(),
        "POST /",
        "",
        r#"{"hello":"world", "foo": "bar"}"#,
    );
    assert_eq!("HTTP/1.1 200 OK\r\n", status);
}

#[test]
fn test_match_body_with_partial_json_string_and_extra_fields() {
    let mut s = Server::new();
    s.mock("POST", "/")
        .match_body(Matcher::PartialJsonString(
            "{\"foo\": \"bar\", \"hello\": \"world\"}".to_string(),
        ))
        .create();

    let (status, _, _) =
        request_with_body(&s.host_with_port(), "POST /", "", r#"{"hello":"world"}"#);
    assert_eq!("HTTP/1.1 501 Not Implemented\r\n", status);
}

#[test]
fn test_mock_with_status() {
    let mut s = Server::new();
    s.mock("GET", "/").with_status(204).with_body("").create();

    let (status_line, _, _) = request(&s.host_with_port(), "GET /", "");
    assert_eq!("HTTP/1.1 204 No Content\r\n", status_line);
}

#[test]
fn test_mock_with_custom_status() {
    let mut s = Server::new();
    s.mock("GET", "/").with_status(499).with_body("").create();

    let (status_line, _, _) = request(&s.host_with_port(), "GET /", "");
    assert_eq!("HTTP/1.1 499 <none>\r\n", status_line);
}

#[test]
fn test_mock_with_body() {
    let mut s = Server::new();
    s.mock("GET", "/").with_body("hello").create();

    let (_, _, body) = request(&s.host_with_port(), "GET /", "");
    assert_eq!("hello", body);
}

#[test]
fn test_mock_with_fn_body() {
    let mut s = Server::new();
    s.mock("GET", "/")
        .with_chunked_body(|w| {
            w.write_all(b"hel")?;
            w.write_all(b"lo")
        })
        .create();

    let (_, _, body) = request(&s.host_with_port(), "GET /", "");
    assert_eq!("hello", body);
}

#[test]
fn test_mock_with_fn_body_streamed_forever() {
    let mut s = Server::new();
    s.mock("GET", "/")
        .with_chunked_body(|w| loop {
            w.write_all(b"spam")?
        })
        .create();

    let stream = request_stream("1.1", s.host_with_port(), "GET /", "", "");
    let (status_line, _, _) = parse_stream(stream, true);
    assert_eq!("HTTP/1.1 200 OK\r\n", status_line);
}

#[test]
fn test_mock_with_body_from_request() {
    let mut s = Server::new();
    s.mock("GET", Matcher::Any)
        .with_body_from_request(|request| {
            if request.path() == "/world" {
                "hello world".into()
            } else {
                "just hello".into()
            }
        })
        .create();

    let (_, _, body) = request(&s.host_with_port(), "GET /world", "");
    assert_eq!("hello world", body);

    let (_, _, body) = request(&s.host_with_port(), "GET /", "");
    assert_eq!("just hello", body);
}

#[test]
fn test_mock_with_body_from_request_body() {
    let mut s = Server::new();
    s.mock("GET", "/")
        .with_body_from_request(|request| {
            let body = std::str::from_utf8(request.body().unwrap()).unwrap();
            if body == "test" {
                "test".into()
            } else {
                "not a test".into()
            }
        })
        .create();

    let (_, _, body) = request_with_body(&s.host_with_port(), "GET /", "", "test");
    assert_eq!("test", body);

    let (_, _, body) = request_with_body(&s.host_with_port(), "GET /", "", "something else");
    assert_eq!("not a test", body);
}

#[test]
fn test_mock_with_header() {
    let mut s = Server::new();
    s.mock("GET", "/")
        .with_header("content-type", "application/json")
        .with_body("{}")
        .create();

    let (_, headers, _) = request(&s.host_with_port(), "GET /", "");
    assert!(headers.contains(&"content-type: application/json".to_string()));
}

#[test]
fn test_mock_with_multiple_headers() {
    let mut s = Server::new();
    s.mock("GET", "/")
        .with_header("content-type", "application/json")
        .with_header("x-api-key", "1234")
        .with_body("{}")
        .create();

    let (_, headers, _) = request(&s.host_with_port(), "GET /", "");
    assert!(headers.contains(&"content-type: application/json".to_string()));
    assert!(headers.contains(&"x-api-key: 1234".to_string()));
}

#[test]
fn test_mock_preserves_header_order() {
    let mut s = Server::new();
    let mut expected_headers = Vec::new();
    let mut mock = s.mock("GET", "/");

    // Add a large number of headers so getting the same order accidentally is unlikely.
    for i in 0..100 {
        let field = format!("x-custom-header-{}", i);
        let value = "test";
        mock = mock.with_header(&field, value);
        expected_headers.push(format!("{}: {}", field, value));
    }

    mock.create();

    let (_, headers, _) = request(&s.host_with_port(), "GET /", "");
    let custom_headers: Vec<_> = headers
        .into_iter()
        .filter(|header| header.starts_with("x-custom-header"))
        .collect();

    assert_eq!(custom_headers, expected_headers);
}

#[test]
fn test_pooled_server_going_out_of_context_removes_all_mocks() {
    let address;

    {
        let mut s = Server::new();
        address = s.host_with_port();

        s.mock("GET", "/reset").create();

        let (working_status_line, _, _) = request(&s.host_with_port(), "GET /reset", "");
        assert_eq!("HTTP/1.1 200 OK\r\n", working_status_line);
    }

    let (reset_status_line, _, _) = request(address, "GET /reset", "");
    assert_eq!("HTTP/1.1 501 Not Implemented\r\n", reset_status_line);
}

#[test]
fn test_unpooled_server_going_out_of_context_removes_all_mocks() {
    let address;

    {
        let mut s = Server::new_with_port(0);
        address = s.host_with_port();

        s.mock("GET", "/reset").create();

        let (working_status_line, _, _) = request(&s.host_with_port(), "GET /reset", "");
        assert_eq!("HTTP/1.1 200 OK\r\n", working_status_line);
    }

    let (reset_status_line, _, _) = request(address, "GET /reset", "");
    assert_eq!("HTTP/1.1 501 Not Implemented\r\n", reset_status_line);
}

#[test]
fn test_remove_a_single_mock() {
    let mut s = Server::new();

    let m1 = s.mock("GET", "/").create();
    m1.remove();

    let (status_line, _, _) = request(&s.host_with_port(), "GET /", "");
    assert_eq!("HTTP/1.1 501 Not Implemented\r\n", status_line);
}

#[test]
fn test_regex_match_path() {
    let mut s = Server::new();
    s.mock("GET", Matcher::Regex(r"^/a/\d{1}$".to_string()))
        .with_body("aaa")
        .create();
    s.mock("GET", Matcher::Regex(r"^/b/\d{1}$".to_string()))
        .with_body("bbb")
        .create();

    let (_, _, body_a) = request(&s.host_with_port(), "GET /a/1", "");
    assert_eq!("aaa", body_a);

    let (_, _, body_b) = request(&s.host_with_port(), "GET /b/2", "");
    assert_eq!("bbb", body_b);

    let (status_line, _, _) = request(&s.host_with_port(), "GET /a/11", "");
    assert_eq!("HTTP/1.1 501 Not Implemented\r\n", status_line);

    let (status_line, _, _) = request(&s.host_with_port(), "GET /c/2", "");
    assert_eq!("HTTP/1.1 501 Not Implemented\r\n", status_line);
}

#[test]
fn test_regex_match_header() {
    let mut s = Server::new();
    s.mock("GET", "/")
        .match_header(
            "Authorization",
            Matcher::Regex(r"^Bearer token\.\w+$".to_string()),
        )
        .with_body("{}")
        .create();

    let (_, _, body_json) = request(
        &s.host_with_port(),
        "GET /",
        "Authorization: Bearer token.payload\r\n",
    );
    assert_eq!("{}", body_json);

    let (status_line, _, _) = request(
        &s.host_with_port(),
        "GET /",
        "authorization: Beare none\r\n",
    );
    assert_eq!("HTTP/1.1 501 Not Implemented\r\n", status_line);
}

#[test]
fn test_any_of_match_header() {
    let mut s = Server::new();
    s.mock("GET", "/")
        .match_header(
            "Via",
            Matcher::AnyOf(vec![
                Matcher::Exact("one".into()),
                Matcher::Exact("two".into()),
            ]),
        )
        .with_body("{}")
        .create();

    let (_, _, body_json) = request(&s.host_with_port(), "GET /", "Via: one\r\n");
    assert_eq!("{}", body_json);

    let (_, _, body_json) = request(&s.host_with_port(), "GET /", "Via: two\r\n");
    assert_eq!("{}", body_json);

    let (_, _, body_json) = request(&s.host_with_port(), "GET /", "Via: one\r\nVia: two\r\n");
    assert_eq!("{}", body_json);

    let (status_line, _, _) = request(
        &s.host_with_port(),
        "GET /",
        "Via: one\r\nVia: two\r\nVia: wrong\r\n",
    );
    assert!(status_line.starts_with("HTTP/1.1 501 "));

    let (status_line, _, _) = request(&s.host_with_port(), "GET /", "Via: wrong\r\n");
    assert!(status_line.starts_with("HTTP/1.1 501 "));
}

#[test]
fn test_any_of_match_body() {
    let mut s = Server::new();
    s.mock("GET", "/")
        .match_body(Matcher::AnyOf(vec![
            Matcher::Regex("one".to_string()),
            Matcher::Regex("two".to_string()),
        ]))
        .create();

    let (status_line, _, _) = request_with_body(&s.host_with_port(), "GET /", "", "one");
    assert!(status_line.starts_with("HTTP/1.1 200 "));

    let (status_line, _, _) = request_with_body(&s.host_with_port(), "GET /", "", "two");
    assert!(status_line.starts_with("HTTP/1.1 200 "));

    let (status_line, _, _) = request_with_body(&s.host_with_port(), "GET /", "", "one two");
    assert!(status_line.starts_with("HTTP/1.1 200 "));

    let (status_line, _, _) = request_with_body(&s.host_with_port(), "GET /", "", "three");
    assert!(status_line.starts_with("HTTP/1.1 501 "));
}

#[test]
fn test_any_of_missing_match_header() {
    let mut s = Server::new();
    s.mock("GET", "/")
        .match_header(
            "Via",
            Matcher::AnyOf(vec![Matcher::Exact("one".into()), Matcher::Missing]),
        )
        .with_body("{}")
        .create();

    let (_, _, body_json) = request(&s.host_with_port(), "GET /", "Via: one\r\n");
    assert_eq!("{}", body_json);

    let (_, _, body_json) = request(
        &s.host_with_port(),
        "GET /",
        "Via: one\r\nVia: one\r\nVia: one\r\n",
    );
    assert_eq!("{}", body_json);

    let (_, _, body_json) = request(&s.host_with_port(), "GET /", "NotVia: one\r\n");
    assert_eq!("{}", body_json);

    let (status_line, _, _) = request(&s.host_with_port(), "GET /", "Via: wrong\r\n");
    assert!(status_line.starts_with("HTTP/1.1 501 "));

    let (status_line, _, _) = request(&s.host_with_port(), "GET /", "Via: wrong\r\nVia: one\r\n");
    assert!(status_line.starts_with("HTTP/1.1 501 "));

    let (status_line, _, _) = request(&s.host_with_port(), "GET /", "Via: one\r\nVia: wrong\r\n");
    assert!(status_line.starts_with("HTTP/1.1 501 "));
}

#[test]
fn test_all_of_match_header() {
    let mut s = Server::new();
    s.mock("GET", "/")
        .match_header(
            "Via",
            Matcher::AllOf(vec![
                Matcher::Regex("one".into()),
                Matcher::Regex("two".into()),
            ]),
        )
        .with_body("{}")
        .create();

    let (status_line, _, _) = request(&s.host_with_port(), "GET /", "Via: one\r\n");
    assert!(status_line.starts_with("HTTP/1.1 501 "));

    let (status_line, _, _) = request(&s.host_with_port(), "GET /", "Via: two\r\n");
    assert!(status_line.starts_with("HTTP/1.1 501 "));

    let (status_line, _, _) = request(
        &s.host_with_port(),
        "GET /",
        "Via: one two\r\nVia: one two three\r\n",
    );
    assert!(status_line.starts_with("HTTP/1.1 200 "));

    let (status_line, _, _) = request(
        &s.host_with_port(),
        "GET /",
        "Via: one\r\nVia: two\r\nVia: wrong\r\n",
    );
    assert!(status_line.starts_with("HTTP/1.1 501 "));

    let (status_line, _, _) = request(&s.host_with_port(), "GET /", "Via: wrong\r\n");
    assert!(status_line.starts_with("HTTP/1.1 501 "));
}

#[test]
fn test_all_of_match_body() {
    let mut s = Server::new();
    s.mock("GET", "/")
        .match_body(Matcher::AllOf(vec![
            Matcher::Regex("one".to_string()),
            Matcher::Regex("two".to_string()),
        ]))
        .create();

    let (status_line, _, _) = request_with_body(&s.host_with_port(), "GET /", "", "one");
    assert!(status_line.starts_with("HTTP/1.1 501 "));

    let (status_line, _, _) = request_with_body(&s.host_with_port(), "GET /", "", "two");
    assert!(status_line.starts_with("HTTP/1.1 501 "));

    let (status_line, _, _) = request_with_body(&s.host_with_port(), "GET /", "", "one two");
    assert!(status_line.starts_with("HTTP/1.1 200 "));

    let (status_line, _, _) = request_with_body(&s.host_with_port(), "GET /", "", "three");
    assert!(status_line.starts_with("HTTP/1.1 501 "));
}

#[test]
fn test_all_of_missing_match_header() {
    let mut s = Server::new();
    s.mock("GET", "/")
        .match_header("Via", Matcher::AllOf(vec![Matcher::Missing]))
        .with_body("{}")
        .create();

    let (status_line, _, _) = request(&s.host_with_port(), "GET /", "Via: one\r\n");
    assert!(status_line.starts_with("HTTP/1.1 501 "));

    let (status_line, _, _) = request(
        &s.host_with_port(),
        "GET /",
        "Via: one\r\nVia: one\r\nVia: one\r\n",
    );
    assert!(status_line.starts_with("HTTP/1.1 501 "));

    let (status_line, _, _) = request(&s.host_with_port(), "GET /", "NotVia: one\r\n");
    assert!(status_line.starts_with("HTTP/1.1 200 "));

    let (status_line, _, _) = request(&s.host_with_port(), "GET /", "Via: wrong\r\n");
    assert!(status_line.starts_with("HTTP/1.1 501 "));

    let (status_line, _, _) = request(&s.host_with_port(), "GET /", "Via: wrong\r\nVia: one\r\n");
    assert!(status_line.starts_with("HTTP/1.1 501 "));

    let (status_line, _, _) = request(&s.host_with_port(), "GET /", "Via: one\r\nVia: wrong\r\n");
    assert!(status_line.starts_with("HTTP/1.1 501 "));
}

#[test]
fn test_large_utf8_body() {
    let mut s = Server::new();
    let mock_body: String = rand::thread_rng()
        .sample_iter(&Alphanumeric)
        .map(char::from)
        .take(3 * 1024) // Must be larger than the request read buffer
        .map(char::from)
        .collect();

    s.mock("GET", "/").with_body(&mock_body).create();

    let (_, _, body) = request(&s.host_with_port(), "GET /", "");
    assert_eq!(mock_body, body);
}

#[test]
fn test_body_from_file() {
    let mut s = Server::new();
    s.mock("GET", "/")
        .with_body_from_file("tests/files/simple.http")
        .create();
    let (status_line, _, body) = request(&s.host_with_port(), "GET /", "");
    assert_eq!("HTTP/1.1 200 OK\r\n", status_line);
    assert_eq!("test body\n", body);
}

#[test]
fn test_display_mock_matching_exact_path() {
    let mut s = Server::new();
    let mock = s.mock("GET", "/hello");

    assert_eq!("\r\nGET /hello\r\n", format!("{}", mock));
}

#[test]
fn test_display_mock_matching_regex_path() {
    let mut s = Server::new();
    let mock = s.mock("GET", Matcher::Regex(r"^/hello/\d+$".to_string()));

    assert_eq!("\r\nGET ^/hello/\\d+$ (regex)\r\n", format!("{}", mock));
}

#[test]
fn test_display_mock_matching_any_path() {
    let mut s = Server::new();
    let mock = s.mock("GET", Matcher::Any);

    assert_eq!("\r\nGET (any)\r\n", format!("{}", mock));
}

#[test]
fn test_display_mock_matching_exact_query() {
    let mut s = Server::new();
    let mock = s.mock("GET", "/test?hello=world");

    assert_eq!("\r\nGET /test?hello=world\r\n", format!("{}", mock));
}

#[test]
fn test_display_mock_matching_regex_query() {
    let mut s = Server::new();
    let mock = s
        .mock("GET", "/test")
        .match_query(Matcher::Regex("hello=world".to_string()));

    assert_eq!("\r\nGET /test?hello=world (regex)\r\n", format!("{}", mock));
}

#[test]
fn test_display_mock_matching_any_query() {
    let mut s = Server::new();
    let mock = s.mock("GET", "/test").match_query(Matcher::Any);

    assert_eq!("\r\nGET /test?(any)\r\n", format!("{}", mock));
}

#[test]
fn test_display_mock_matching_exact_header() {
    let mut s = Server::new();
    let mock = s
        .mock("GET", "/")
        .match_header("content-type", "text")
        .create();

    assert_eq!("\r\nGET /\r\ncontent-type: text\r\n", format!("{}", mock));
}

#[test]
fn test_display_mock_matching_multiple_headers() {
    let mut s = Server::new();
    let mock = s
        .mock("GET", "/")
        .match_header("content-type", "text")
        .match_header("content-length", Matcher::Regex(r"\d+".to_string()))
        .match_header("authorization", Matcher::Any)
        .match_header("x-request-id", Matcher::Missing)
        .create();

    assert_eq!("\r\nGET /\r\ncontent-type: text\r\ncontent-length: \\d+ (regex)\r\nauthorization: (any)\r\nx-request-id: (missing)\r\n", format!("{}", mock));
}

#[test]
fn test_display_mock_matching_exact_body() {
    let mut s = Server::new();
    let mock = s.mock("POST", "/").match_body("hello").create();

    assert_eq!("\r\nPOST /\r\nhello\r\n", format!("{}", mock));
}

#[test]
fn test_display_mock_matching_regex_body() {
    let mut s = Server::new();
    let mock = s
        .mock("POST", "/")
        .match_body(Matcher::Regex("hello".to_string()))
        .create();

    assert_eq!("\r\nPOST /\r\nhello\r\n", format!("{}", mock));
}

#[test]
fn test_display_mock_matching_any_body() {
    let mut s = Server::new();
    let mock = s.mock("POST", "/").match_body(Matcher::Any).create();

    assert_eq!("\r\nPOST /\r\n", format!("{}", mock));
}

#[test]
fn test_display_mock_matching_headers_and_body() {
    let mut s = Server::new();
    let mock = s
        .mock("POST", "/")
        .match_header("content-type", "text")
        .match_body("hello")
        .create();

    assert_eq!(
        "\r\nPOST /\r\ncontent-type: text\r\nhello\r\n",
        format!("{}", mock)
    );
}

#[test]
fn test_display_mock_matching_all_of_queries() {
    let mut s = Server::new();
    let mock = s
        .mock("POST", "/")
        .match_query(Matcher::AllOf(vec![
            Matcher::Exact("query1".to_string()),
            Matcher::UrlEncoded("key".to_string(), "val".to_string()),
        ]))
        .create();

    assert_eq!(
        "\r\nPOST /?(query1, key=val (urlencoded)) (all of)\r\n",
        format!("{}", mock)
    );
}

#[test]
fn test_display_mock_matching_any_of_headers() {
    let mut s = Server::new();
    let mock = s
        .mock("POST", "/")
        .match_header(
            "content-type",
            Matcher::AnyOf(vec![
                Matcher::Exact("type1".to_string()),
                Matcher::Regex("type2".to_string()),
            ]),
        )
        .create();

    assert_eq!(
        "\r\nPOST /\r\ncontent-type: (type1, type2 (regex)) (any of)\r\n",
        format!("{}", mock)
    );
}

#[test]
fn test_assert_defaults_to_one_hit() {
    let mut s = Server::new();
    let host = s.host_with_port();
    let mock = s.mock("GET", "/hello").create();

    request(&host, "GET /hello", "");

    mock.assert();
}

#[test]
fn test_expect() {
    let mut s = Server::new();
    let host = s.host_with_port();
    let mock = s.mock("GET", "/hello").expect(3).create();

    request(&host, "GET /hello", "");
    request(&host, "GET /hello", "");
    request(&host, "GET /hello", "");

    mock.assert();
}

#[test]
fn test_expect_at_least_and_at_most() {
    let mut s = Server::new();
    let host = s.host_with_port();
    let mock = s
        .mock("GET", "/hello")
        .expect_at_least(3)
        .expect_at_most(6)
        .create();

    request(&host, "GET /hello", "");
    request(&host, "GET /hello", "");
    request(&host, "GET /hello", "");
    request(&host, "GET /hello", "");
    request(&host, "GET /hello", "");

    mock.assert();
}

#[test]
fn test_expect_at_least() {
    let mut s = Server::new();
    let host = s.host_with_port();
    let mock = s.mock("GET", "/hello").expect_at_least(3).create();

    request(&host, "GET /hello", "");
    request(&host, "GET /hello", "");
    request(&host, "GET /hello", "");

    mock.assert();
}

#[test]
fn test_expect_at_least_more() {
    let mut s = Server::new();
    let host = s.host_with_port();
    let mock = s.mock("GET", "/hello").expect_at_least(3).create();

    request(&host, "GET /hello", "");
    request(&host, "GET /hello", "");
    request(&host, "GET /hello", "");
    request(&host, "GET /hello", "");

    mock.assert();
}

#[test]
fn test_expect_at_most_with_needed_requests() {
    let mut s = Server::new();
    let host = s.host_with_port();
    let mock = s.mock("GET", "/hello").expect_at_most(3).create();

    request(&host, "GET /hello", "");
    request(&host, "GET /hello", "");
    request(&host, "GET /hello", "");

    mock.assert();
}

#[test]
fn test_expect_at_most_with_few_requests() {
    let mut s = Server::new();
    let host = s.host_with_port();
    let mock = s.mock("GET", "/hello").expect_at_most(3).create();

    request(&host, "GET /hello", "");
    request(&host, "GET /hello", "");

    mock.assert();
}

#[test]
#[should_panic(
    expected = "\n> Expected at least 3 request(s) to:\n\r\nGET /hello\r\n\n...but received 2\n"
)]
fn test_assert_panics_expect_at_least_with_too_few_requests() {
    let mut s = Server::new();
    let host = s.host_with_port();
    let mock = s.mock("GET", "/hello").expect_at_least(3).create();

    request(&host, "GET /hello", "");
    request(&host, "GET /hello", "");

    mock.assert();
}

#[test]
#[should_panic(
    expected = "\n> Expected at most 3 request(s) to:\n\r\nGET /hello\r\n\n...but received 4\n"
)]
fn test_assert_panics_expect_at_most_with_too_many_requests() {
    let mut s = Server::new();
    let host = s.host_with_port();
    let mock = s.mock("GET", "/hello").expect_at_most(3).create();

    request(&host, "GET /hello", "");
    request(&host, "GET /hello", "");
    request(&host, "GET /hello", "");
    request(&host, "GET /hello", "");

    mock.assert();
}

#[test]
#[should_panic(
    expected = "\n> Expected between 3 and 5 request(s) to:\n\r\nGET /hello\r\n\n...but received 2\n"
)]
fn test_assert_panics_expect_at_least_and_at_most_with_too_few_requests() {
    let mut s = Server::new();
    let host = s.host_with_port();
    let mock = s
        .mock("GET", "/hello")
        .expect_at_least(3)
        .expect_at_most(5)
        .create();

    request(&host, "GET /hello", "");
    request(&host, "GET /hello", "");

    mock.assert();
}

#[test]
#[should_panic(
    expected = "\n> Expected between 3 and 5 request(s) to:\n\r\nGET /hello\r\n\n...but received 6\n"
)]
fn test_assert_panics_expect_at_least_and_at_most_with_too_many_requests() {
    let mut s = Server::new();
    let host = s.host_with_port();
    let mock = s
        .mock("GET", "/hello")
        .expect_at_least(3)
        .expect_at_most(5)
        .create();

    request(&host, "GET /hello", "");
    request(&host, "GET /hello", "");
    request(&host, "GET /hello", "");
    request(&host, "GET /hello", "");
    request(&host, "GET /hello", "");
    request(&host, "GET /hello", "");

    mock.assert();
}

#[test]
#[should_panic(expected = "\n> Expected 1 request(s) to:\n\r\nGET /hello\r\n\n...but received 0\n")]
fn test_assert_panics_if_no_request_was_performed() {
    let mut s = Server::new();
    let mock = s.mock("GET", "/hello").create();

    mock.assert();
}

#[test]
#[should_panic(expected = "\n> Expected 3 request(s) to:\n\r\nGET /hello\r\n\n...but received 2\n")]
fn test_assert_panics_with_too_few_requests() {
    let mut s = Server::new();
    let host = s.host_with_port();
    let mock = s.mock("GET", "/hello").expect(3).create();

    request(&host, "GET /hello", "");
    request(&host, "GET /hello", "");

    mock.assert();
}

#[test]
#[should_panic(expected = "\n> Expected 3 request(s) to:\n\r\nGET /hello\r\n\n...but received 4\n")]
fn test_assert_panics_with_too_many_requests() {
    let mut s = Server::new();
    let host = s.host_with_port();
    let mock = s.mock("GET", "/hello").expect(3).create();

    request(&host, "GET /hello", "");
    request(&host, "GET /hello", "");
    request(&host, "GET /hello", "");
    request(&host, "GET /hello", "");

    mock.assert();
}

#[test]
#[should_panic(
    expected = "\n> Expected 1 request(s) to:\n\r\nGET /hello\r\n\n...but received 0\n\n> The last unmatched request was:\n\r\nGET /bye\r\n\n> Difference:\n\n\u{1b}[31mGET /hello\n\u{1b}[0m\u{1b}[32mGET\u{1b}[0m\u{1b}[32m \u{1b}[0m\u{1b}[42;30m/bye\u{1b}[0m\u{1b}[32m\n\u{1b}[0m\n\n"
)]
#[cfg(feature = "color")]
fn test_assert_with_last_unmatched_request() {
    let mut s = Server::new();
    let host = s.host_with_port();
    let mock = s.mock("GET", "/hello").create();

    request(&host, "GET /bye", "");

    mock.assert();
}

// Same test but without colors (for Appveyor)
#[test]
#[should_panic(
    expected = "\n> Expected 1 request(s) to:\n\r\nGET /hello\r\n\n...but received 0\n\n> The last unmatched request was:\n\r\nGET /bye\r\n\n> Difference:\n\nGET /hello\nGET /bye\n\n\n"
)]
#[cfg(not(feature = "color"))]
fn test_assert_with_last_unmatched_request() {
    let mut s = Server::new();
    let host = s.host_with_port();
    let mock = s.mock("GET", "/hello").create();

    request(&host, "GET /bye", "");

    mock.assert();
}

#[test]
#[should_panic(
    expected = "\n> Expected 1 request(s) to:\n\r\nGET /hello?world=1\r\n\n...but received 0\n\n> The last unmatched request was:\n\r\nGET /hello?world=2\r\n\n> Difference:\n\n\u{1b}[31mGET /hello?world=1\n\u{1b}[0m\u{1b}[32mGET\u{1b}[0m\u{1b}[32m \u{1b}[0m\u{1b}[42;30m/hello?world=2\u{1b}[0m\u{1b}[32m\n\u{1b}[0m\n\n"
)]
#[cfg(feature = "color")]
fn test_assert_with_last_unmatched_request_and_query() {
    let mut s = Server::new();
    let host = s.host_with_port();
    let mock = s.mock("GET", "/hello?world=1").create();

    request(&host, "GET /hello?world=2", "");

    mock.assert();
}

// Same test but without colors (for Appveyor)
#[test]
#[should_panic(
    expected = "\n> Expected 1 request(s) to:\n\r\nGET /hello?world=1\r\n\n...but received 0\n\n> The last unmatched request was:\n\r\nGET /hello?world=2\r\n\n> Difference:\n\nGET /hello?world=1\nGET /hello?world=2\n\n\n"
)]
#[cfg(not(feature = "color"))]
fn test_assert_with_last_unmatched_request_and_query() {
    let mut s = Server::new();
    let host = s.host_with_port();
    let mock = s.mock("GET", "/hello?world=1").create();

    request(&host, "GET /hello?world=2", "");

    mock.assert();
}

#[test]
#[should_panic(
    expected = "\n> Expected 1 request(s) to:\n\r\nGET /hello\r\n\n...but received 0\n\n> The last unmatched request was:\n\r\nGET /bye\r\nauthorization: 1234\r\naccept: text\r\n\n> Difference:\n\n\u{1b}[31mGET /hello\n\u{1b}[0m\u{1b}[32mGET\u{1b}[0m\u{1b}[32m \u{1b}[0m\u{1b}[42;30m/bye\u{1b}[0m\u{1b}[32m\n\u{1b}[0m\u{1b}[92mauthorization: 1234\n\u{1b}[0m\u{1b}[92maccept: text\n\u{1b}[0m\n\n"
)]
#[cfg(feature = "color")]
fn test_assert_with_last_unmatched_request_and_headers() {
    let mut s = Server::new();
    let host = s.host_with_port();
    let mock = s.mock("GET", "/hello").create();

    request(&host, "GET /bye", "authorization: 1234\r\naccept: text\r\n");

    mock.assert();
}

// Same test but without colors (for Appveyor)
#[test]
#[should_panic(
    expected = "\n> Expected 1 request(s) to:\n\r\nGET /hello\r\n\n...but received 0\n\n> The last unmatched request was:\n\r\nGET /bye\r\nauthorization: 1234\r\naccept: text\r\n\n> Difference:\n\nGET /hello\nGET /bye\nauthorization: 1234\naccept: text\n\n\n"
)]
#[cfg(not(feature = "color"))]
fn test_assert_with_last_unmatched_request_and_headers() {
    let mut s = Server::new();
    let host = s.host_with_port();
    let mock = s.mock("GET", "/hello").create();

    request(&host, "GET /bye", "authorization: 1234\r\naccept: text\r\n");

    mock.assert();
}

#[test]
#[should_panic(
    expected = "\n> Expected 1 request(s) to:\n\r\nGET /hello\r\n\n...but received 0\n\n> The last unmatched request was:\n\r\nPOST /bye\r\ncontent-length: 5\r\nhello\r\n\n"
)]
fn test_assert_with_last_unmatched_request_and_body() {
    let mut s = Server::new();
    let host = s.host_with_port();
    let mock = s.mock("GET", "/hello").create();

    request_with_body(&host, "POST /bye", "", "hello");

    mock.assert();
}

#[test]
fn test_request_from_thread() {
    let mut s = Server::new();
    let host = s.host_with_port();
    let mock = s.mock("GET", "/").create();

    let process = thread::spawn(move || {
        request(&host, "GET /", "");
    });

    process.join().unwrap();

    mock.assert();
}

#[test]
fn test_mock_from_inside_thread_does_not_lock_forever() {
    let server = Arc::new(Mutex::new(Server::new()));
    let host;

    {
        let s1_mutex = Arc::clone(&server);
        let mut s1 = s1_mutex.lock().unwrap();
        host = s1.host_with_port();
        s1.mock("GET", "/").with_body("outside").create();
    }

    let s2_mutex = Arc::clone(&server);
    let process = thread::spawn(move || {
        let mut s2 = s2_mutex.lock().unwrap();
        s2.mock("GET", "/").with_body("inside").create();
    });

    process.join().unwrap();

    let (status_line, _, body) = request(&host, "GET /", "");
    assert!(status_line.starts_with("HTTP/1.1 200 "));
    assert_eq!("outside", body);
}

#[test]
fn test_head_request_with_overridden_content_length() {
    let mut s = Server::new();
    let host = s.host_with_port();
    s.mock("HEAD", "/")
        .with_header("content-length", "100")
        .create();

    let (_, headers, _) = request(&host, "HEAD /", "");

    assert_eq!(
        vec!["connection: close", "content-length: 100"],
        headers[0..=1]
    );
}

#[test]
fn test_propagate_protocol_to_response() {
    let mut s = Server::new();
    let host = s.host_with_port();
    s.mock("GET", "/").create();

    let stream = request_stream("1.0", &host, "GET /", "", "");

    let (status_line, _, _) = parse_stream(stream, true);
    assert_eq!("HTTP/1.0 200 OK\r\n", status_line);
}

#[test]
fn test_large_body_without_content_length() {
    let mut s = Server::new();
    let host = s.host_with_port();
    let body = "123".repeat(2048);

    s.mock("POST", "/").match_body(body.as_str()).create();

    let headers = format!("content-length: {}\r\n", body.len());
    let stream = request_stream("1.0", &host, "POST /", &headers, &body);

    let (status_line, _, _) = parse_stream(stream, false);
    assert_eq!("HTTP/1.0 200 OK\r\n", status_line);
}

#[test]
fn test_transfer_encoding_chunked() {
    let mut s = Server::new();
    let host = s.host_with_port();
    s.mock("POST", "/")
        .match_body("Hello, chunked world!")
        .create();

    let body = "3\r\nHel\r\n5\r\nlo, c\r\nD\r\nhunked world!\r\n0\r\n\r\n";

    let (status, _, _) = parse_stream(
        request_stream(
            "1.1",
            &host,
            "POST /",
            "Transfer-Encoding: chunked\r\n",
            body,
        ),
        false,
    );

    assert_eq!("HTTP/1.1 200 OK\r\n", status);
}

#[test]
fn test_match_exact_query() {
    let mut s = Server::new();
    let host = s.host_with_port();
    s.mock("GET", "/hello")
        .match_query(Matcher::Exact("number=one".to_string()))
        .create();

    let (status_line, _, _) = request(&host, "GET /hello?number=one", "");
    assert_eq!("HTTP/1.1 200 OK\r\n", status_line);

    let (status_line, _, _) = request(&host, "GET /hello?number=two", "");
    assert_eq!("HTTP/1.1 501 Not Implemented\r\n", status_line);
}

#[test]
fn test_match_exact_query_via_path() {
    let mut s = Server::new();
    let host = s.host_with_port();
    s.mock("GET", "/hello?number=one").create();

    let (status_line, _, _) = request(&host, "GET /hello?number=one", "");
    assert_eq!("HTTP/1.1 200 OK\r\n", status_line);

    let (status_line, _, _) = request(&host, "GET /hello?number=two", "");
    assert_eq!("HTTP/1.1 501 Not Implemented\r\n", status_line);
}

#[test]
fn test_match_partial_query_by_regex() {
    let mut s = Server::new();
    let host = s.host_with_port();
    s.mock("GET", "/hello")
        .match_query(Matcher::Regex("number=one".to_string()))
        .create();

    let (status_line, _, _) = request(&host, "GET /hello?something=else&number=one", "");
    assert_eq!("HTTP/1.1 200 OK\r\n", status_line);
}

#[test]
fn test_match_partial_query_by_urlencoded() {
    let mut s = Server::new();
    let host = s.host_with_port();
    s.mock("GET", "/hello")
        .match_query(Matcher::UrlEncoded("num ber".into(), "o ne".into()))
        .create();

    let (status_line, _, _) = request(&host, "GET /hello?something=else&num%20ber=o%20ne", "");
    assert_eq!("HTTP/1.1 200 OK\r\n", status_line);

    let (status_line, _, _) = request(&host, "GET /hello?something=else&number=one", "");
    assert_eq!("HTTP/1.1 501 Not Implemented\r\n", status_line);
}

#[test]
fn test_match_partial_query_by_regex_all_of() {
    let mut s = Server::new();
    let host = s.host_with_port();
    s.mock("GET", "/hello")
        .match_query(Matcher::AllOf(vec![
            Matcher::Regex("number=one".to_string()),
            Matcher::Regex("hello=world".to_string()),
        ]))
        .create();

    let (status_line, _, _) = request(
        &host,
        "GET /hello?hello=world&something=else&number=one",
        "",
    );
    assert_eq!("HTTP/1.1 200 OK\r\n", status_line);

    let (status_line, _, _) = request(&host, "GET /hello?hello=world&something=else", "");
    assert_eq!("HTTP/1.1 501 Not Implemented\r\n", status_line);
}

#[test]
fn test_match_partial_query_by_urlencoded_all_of() {
    let mut s = Server::new();
    let host = s.host_with_port();
    s.mock("GET", "/hello")
        .match_query(Matcher::AllOf(vec![
            Matcher::UrlEncoded("num ber".into(), "o ne".into()),
            Matcher::UrlEncoded("hello".into(), "world".into()),
        ]))
        .create();

    let (status_line, _, _) = request(
        &host,
        "GET /hello?hello=world&something=else&num%20ber=o%20ne",
        "",
    );
    assert_eq!("HTTP/1.1 200 OK\r\n", status_line);

    let (status_line, _, _) = request(&host, "GET /hello?hello=world&something=else", "");
    assert_eq!("HTTP/1.1 501 Not Implemented\r\n", status_line);
}

#[test]
fn test_match_query_with_non_percent_url_escaping() {
    let mut s = Server::new();
    let host = s.host_with_port();
    s.mock("GET", "/hello")
        .match_query(Matcher::AllOf(vec![
            Matcher::UrlEncoded("num ber".into(), "o ne".into()),
            Matcher::UrlEncoded("hello".into(), "world".into()),
        ]))
        .create();

    let (status_line, _, _) = request(
        &host,
        "GET /hello?hello=world&something=else&num+ber=o+ne",
        "",
    );
    assert_eq!("HTTP/1.1 200 OK\r\n", status_line);
}

#[test]
fn test_match_missing_query() {
    let mut s = Server::new();
    let host = s.host_with_port();
    s.mock("GET", "/hello")
        .match_query(Matcher::Missing)
        .create();

    let (status_line, _, _) = request(&host, "GET /hello?", "");
    assert_eq!("HTTP/1.1 200 OK\r\n", status_line);

    let (status_line, _, _) = request(&host, "GET /hello?number=one", "");
    assert_eq!("HTTP/1.1 501 Not Implemented\r\n", status_line);
}

#[test]
fn test_match_any_query() {
    let mut s = Server::new();
    let host = s.host_with_port();
    s.mock("GET", "/hello").match_query(Matcher::Any).create();

    let (status_line, _, _) = request(&host, "GET /hello", "");
    assert_eq!("HTTP/1.1 200 OK\r\n", status_line);

    let (status_line, _, _) = request(&host, "GET /hello?", "");
    assert_eq!("HTTP/1.1 200 OK\r\n", status_line);

    let (status_line, _, _) = request(&host, "GET /hello?number=one", "");
    assert_eq!("HTTP/1.1 200 OK\r\n", status_line);
}

#[test]
fn test_anyof_exact_path_and_query_matcher() {
    let mut s = Server::new();
    let host = s.host_with_port();
    let mock = s
        .mock(
            "GET",
            Matcher::AnyOf(vec![Matcher::Exact("/hello?world".to_string())]),
        )
        .create();

    let (status_line, _, _) = request(&host, "GET /hello?world", "");
    assert_eq!("HTTP/1.1 200 OK\r\n", status_line);

    mock.assert();
}

#[test]
fn test_default_headers() {
    let mut s = Server::new();
    let host = s.host_with_port();
    s.mock("GET", "/").create();

    let (_, headers, _) = request(&host, "GET /", "");
    assert_eq!(3, headers.len());
    assert_eq!(
        vec!["connection: close", "content-length: 0"],
        headers[0..=1]
    );
    let date_parts: Vec<&str> = headers[2].split(':').collect();
    assert_eq!("date", date_parts[0]);
}

#[test]
fn test_missing_create_bad() {
    testing_logger::setup();

    let mut s = Server::new();
    let m = s.mock("GET", "/");
    drop(m);

    // Expecting one warning
    testing_logger::validate(|captured_logs| {
        let warnings = captured_logs
            .iter()
            .filter(|c| c.level == log::Level::Warn)
            .collect::<Vec<&testing_logger::CapturedLog>>();

        assert_eq!(warnings.len(), 1);
        assert_eq!(
            warnings[0].body,
            "Missing .create() call on mock \r\nGET /\r\n"
        );
        assert_eq!(warnings[0].level, log::Level::Warn);
    });
}

#[test]
fn test_missing_create_good() {
    testing_logger::setup();

    let mut s = Server::new();
    let m = s.mock("GET", "/").create();
    drop(m);

    // No warnings should occur
    testing_logger::validate(|captured_logs| {
        assert_eq!(
            captured_logs
                .iter()
                .filter(|c| c.level == log::Level::Warn)
                .count(),
            0
        );
    });
}

#[test]
fn test_same_endpoint_different_responses() {
    let mut s = Server::new();
    let host = s.host_with_port();

    let mock_200 = s.mock("GET", "/hello").with_status(200).create();
    let mock_404 = s.mock("GET", "/hello").with_status(404).create();
    let mock_500 = s.mock("GET", "/hello").with_status(500).create();

    let response_200 = request(&host, "GET /hello", "");
    let response_404 = request(&host, "GET /hello", "");
    let response_500 = request(&host, "GET /hello", "");

    mock_200.assert();
    mock_404.assert();
    mock_500.assert();

    assert_eq!(response_200.0, "HTTP/1.1 200 OK\r\n");
    assert_eq!(response_404.0, "HTTP/1.1 404 Not Found\r\n");
    assert_eq!(response_500.0, "HTTP/1.1 500 Internal Server Error\r\n");
}

#[test]
fn test_same_endpoint_different_responses_last_one_forever() {
    let mut s = Server::new();
    let host = s.host_with_port();

    let _mock_200 = s.mock("GET", "/hello").with_status(200).create();
    let _mock_404 = s.mock("GET", "/hello").with_status(404).create();
    let _mock_500 = s
        .mock("GET", "/hello")
        .expect_at_least(1)
        .with_status(500)
        .create();

    let response_200 = request(&host, "GET /hello", "");
    let response_404 = request(&host, "GET /hello", "");
    let response_500_1 = request(&host, "GET /hello", "");
    let response_500_2 = request(&host, "GET /hello", "");
    let response_500_3 = request(&host, "GET /hello", "");

    assert_eq!(response_200.0, "HTTP/1.1 200 OK\r\n");
    assert_eq!(response_404.0, "HTTP/1.1 404 Not Found\r\n");
    assert_eq!(response_500_1.0, "HTTP/1.1 500 Internal Server Error\r\n");
    assert_eq!(response_500_2.0, "HTTP/1.1 500 Internal Server Error\r\n");
    assert_eq!(response_500_3.0, "HTTP/1.1 500 Internal Server Error\r\n");
}

#[test]
fn test_matched_bool() {
    let mut s = Server::new();
    let host = s.host_with_port();
    let m = s.mock("GET", "/").create();

    let (_, _, _) = request_with_body(&host, "GET /", "", "");
    m.assert();
    assert!(m.matched(), "matched method returns correctly");

    let (_, _, _) = request_with_body(&host, "GET /", "", "");
    assert!(!m.matched(), "matched method returns correctly");
}

#[test]
fn test_invalid_header_field_name() {
    let mut s = Server::new();
    let host = s.host_with_port();
    s.mock("GET", "/").create();

    let (uppercase_status_line, _, _body) = request(&host, "GET /", "Bad Header: something\r\n");
    assert_eq!("HTTP/1.1 400 Bad Request\r\n", uppercase_status_line);
}

#[test]
fn test_running_multiple_servers() {
    let mut s1 = Server::new();
    let mut s2 = Server::new();
    let mut s3 = Server::new();

    s2.mock("GET", "/").with_body("s2").create();
    s1.mock("GET", "/").with_body("s1").create();
    s3.mock("GET", "/").with_body("s3").create();

    let (_, _, body1) = request_with_body(&s1.host_with_port(), "GET /", "", "");
    let (_, _, body2) = request_with_body(&s2.host_with_port(), "GET /", "", "");
    let (_, _, body3) = request_with_body(&s3.host_with_port(), "GET /", "", "");

    assert!(s1.host_with_port() != s2.host_with_port());
    assert!(s2.host_with_port() != s3.host_with_port());
    assert_eq!("s1", body1);
    assert_eq!("s2", body2);
    assert_eq!("s3", body3);
}

static SERIAL_POOL_TESTS: Mutex<()> = Mutex::new(());
const DEFAULT_POOL_SIZE: usize = if cfg!(target_os = "macos") { 20 } else { 50 };

#[test]
#[allow(clippy::vec_init_then_push)]
fn test_server_pool() {
    // two tests can't monopolize the pool at the same time
    let _lock = SERIAL_POOL_TESTS.lock().unwrap();

    // If the pool is not working, this will hit the file descriptor limit (Too many open files)
    for _ in 0..20 {
        let mut servers = vec![];
        // Anything beyond pool size will block.
        for _ in 0..DEFAULT_POOL_SIZE {
            servers.push(Server::new());

            let s = servers.last_mut().unwrap();
            let m = s.mock("GET", "/pool").create();
            let (_, _, _) = request_with_body(&s.host_with_port(), "GET /pool", "", "");
            m.assert();
        }
    }
}

#[tokio::test(flavor = "multi_thread")]
#[allow(clippy::vec_init_then_push)]
async fn test_server_pool_async() {
    // two tests can't monopolize the pool at the same time
    tokio::task::yield_now().await;
    let _lock = tokio::task::block_in_place(|| SERIAL_POOL_TESTS.lock().unwrap());

    // If the pool is not working, this will hit the file descriptor limit (Too many open files)
    for _ in 0..20 {
        let mut servers = vec![];
        // Anything beyond pool size will block
        for _ in 0..DEFAULT_POOL_SIZE {
            servers.push(Server::new_async().await);

            let s = servers.last_mut().unwrap();
            let m = s.mock("GET", "/pool").create_async().await;
            let (_, _, _) = request_with_body(&s.host_with_port(), "GET /pool", "", "");
            m.assert_async().await;
        }
    }
}

#[tokio::test]
async fn test_http2_requests_async() {
    let mut s = Server::new_async().await;
    let m1 = s.mock("GET", "/").with_body("test").create_async().await;

    let response = reqwest::Client::builder()
        .http2_prior_knowledge()
        .build()
        .unwrap()
        .get(s.url())
        .version(reqwest::Version::HTTP_2)
        .send()
        .await
        .unwrap();

    assert_eq!(200, response.status());
    assert_eq!(reqwest::Version::HTTP_2, response.version());
    assert_eq!("test", response.text().await.unwrap());

    m1.assert_async().await;
}

#[tokio::test]
async fn test_simple_route_mock_async() {
    let mut s = Server::new_async().await;
    s.mock("GET", "/hello")
        .with_body("world")
        .create_async()
        .await;

    let response = reqwest::Client::new()
        .get(format!("{}/hello", s.url()))
        .version(reqwest::Version::HTTP_11)
        .send()
        .await
        .unwrap();

    assert_eq!(200, response.status());
    assert_eq!("world", response.text().await.unwrap());
}

#[tokio::test]
async fn test_several_route_mocks_async() {
    let mut s = Server::new_async().await;
    let m1 = s.mock("GET", "/a").with_body("aaa").create_async();
    let m2 = s.mock("GET", "/b").with_body("bbb").create_async();
    let m3 = s.mock("GET", "/c").with_body("ccc").create_async();
    let m4 = s.mock("GET", "/d").with_body("ddd").create_async();

    let (m1, m2, m3, m4) = futures::join!(m1, m2, m3, m4);

    let response_a = reqwest::Client::new()
        .get(format!("{}/a", s.url()))
        .version(reqwest::Version::HTTP_11)
        .send();

    let response_b = reqwest::Client::new()
        .get(format!("{}/b", s.url()))
        .version(reqwest::Version::HTTP_11)
        .send();

    let response_c = reqwest::Client::new()
        .get(format!("{}/c", s.url()))
        .version(reqwest::Version::HTTP_11)
        .send();

    let response_d = reqwest::Client::new()
        .get(format!("{}/d", s.url()))
        .version(reqwest::Version::HTTP_11)
        .send();

    let (response_a, response_b, response_c, response_d) =
        futures::join!(response_a, response_b, response_c, response_d);

    assert_eq!("aaa", response_a.unwrap().text().await.unwrap());
    assert_eq!("bbb", response_b.unwrap().text().await.unwrap());
    assert_eq!("ccc", response_c.unwrap().text().await.unwrap());
    assert_eq!("ddd", response_d.unwrap().text().await.unwrap());

    m1.assert_async().await;
    m2.assert_async().await;
    m3.assert_async().await;
    m4.assert_async().await;
}

#[tokio::test]
async fn test_match_body_asnyc() {
    let mut s = Server::new_async().await;
    s.mock("POST", "/").match_body("hello").create_async().await;

    let response = reqwest::Client::new()
        .post(s.url())
        .version(reqwest::Version::HTTP_11)
        .body("hello")
        .send()
        .await
        .unwrap();

    assert_eq!(200, response.status());
}

#[tokio::test(flavor = "multi_thread")]
async fn test_join_all_async() {
    tokio::task::yield_now().await;
    let _lock = tokio::task::block_in_place(|| SERIAL_POOL_TESTS.lock().unwrap());

    let futures = (0..10).map(|_| async {
        let mut s = Server::new_async().await;
        let m = s.mock("POST", "/").create_async().await;

        reqwest::Client::new().post(s.url()).send().await.unwrap();
        m.assert_async().await;
    });

    let _results = futures::future::join_all(futures).await;
}

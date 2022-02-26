#[macro_use]
extern crate serde_json;

use mockito::{mock, server_address, Matcher};
use rand::distributions::Alphanumeric;
use rand::Rng;
use std::fs;
use std::io::{BufRead, BufReader, Read, Write};
use std::mem;
use std::net::{Shutdown, TcpStream};
use std::path::Path;
use std::str::FromStr;
use std::thread;

type Binary = Vec<u8>;

fn request_stream<StrOrBytes: AsRef<[u8]>>(
    version: &str,
    route: &str,
    headers: &str,
    body: StrOrBytes,
) -> TcpStream {
    let mut stream = TcpStream::connect(server_address()).unwrap();
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

fn binary_request<StrOrBytes: AsRef<[u8]>>(
    route: &str,
    headers: &str,
    body: StrOrBytes,
) -> (String, Vec<String>, Binary) {
    parse_stream(
        request_stream("1.1", route, headers, body),
        route.starts_with("HEAD"),
    )
}

fn request(route: &str, headers: &str) -> (String, Vec<String>, String) {
    let (status, headers, body) = binary_request(route, headers, "");
    let parsed_body: String = std::str::from_utf8(body.as_slice()).unwrap().to_string();
    (status, headers, parsed_body)
}

fn request_with_body(route: &str, headers: &str, body: &str) -> (String, Vec<String>, String) {
    let headers = format!("{}content-length: {}\r\n", headers, body.len());
    let (status, headers, body) = binary_request(route, &headers, body);
    let parsed_body: String = std::str::from_utf8(body.as_slice()).unwrap().to_string();
    (status, headers, parsed_body)
}

#[test]
fn test_create_starts_the_server() {
    let _m = mock("GET", "/").with_body("hello").create();

    let stream = TcpStream::connect(server_address());
    assert!(stream.is_ok());
}

#[test]
fn test_simple_route_mock() {
    let _m = mock("GET", "/hello").with_body("world").create();

    let (status_line, _, body) = request("GET /hello", "");
    assert_eq!("HTTP/1.1 200 OK\r\n", status_line);
    assert_eq!("world", body);
}

#[test]
fn test_two_route_mocks() {
    let _m1 = mock("GET", "/a").with_body("aaa").create();
    let _m2 = mock("GET", "/b").with_body("bbb").create();

    let (_, _, body_a) = request("GET /a", "");

    assert_eq!("aaa", body_a);
    let (_, _, body_b) = request("GET /b", "");
    assert_eq!("bbb", body_b);
}

#[test]
fn test_no_match_returns_501() {
    let _m = mock("GET", "/").with_body("matched").create();

    let (status_line, headers, _) = request("GET /nope", "");
    assert_eq!("HTTP/1.1 501 Mock Not Found\r\n", status_line);
    assert_eq!(vec!["content-length: 0"], headers);
}

#[test]
fn test_match_header() {
    let _m1 = mock("GET", "/")
        .match_header("content-type", "application/json")
        .with_body("{}")
        .create();

    let _m2 = mock("GET", "/")
        .match_header("content-type", "text/plain")
        .with_body("hello")
        .create();

    let (_, _, body_json) = request("GET /", "content-type: application/json\r\n");
    assert_eq!("{}", body_json);

    let (_, _, body_text) = request("GET /", "content-type: text/plain\r\n");
    assert_eq!("hello", body_text);
}

#[test]
fn test_match_header_is_case_insensitive_on_the_field_name() {
    let _m = mock("GET", "/")
        .match_header("content-type", "text/plain")
        .create();

    let (uppercase_status_line, _, _) = request("GET /", "Content-Type: text/plain\r\n");
    assert_eq!("HTTP/1.1 200 OK\r\n", uppercase_status_line);

    let (lowercase_status_line, _, _) = request("GET /", "content-type: text/plain\r\n");
    assert_eq!("HTTP/1.1 200 OK\r\n", lowercase_status_line);
}

#[test]
fn test_match_multiple_headers() {
    let _m = mock("GET", "/")
        .match_header("Content-Type", "text/plain")
        .match_header("Authorization", "secret")
        .with_body("matched")
        .create();

    let (_, _, body_matching) = request(
        "GET /",
        "content-type: text/plain\r\nauthorization: secret\r\n",
    );
    assert_eq!("matched", body_matching);

    let (status_not_matching, _, _) = request(
        "GET /",
        "content-type: text/plain\r\nauthorization: meh\r\n",
    );
    assert_eq!("HTTP/1.1 501 Mock Not Found\r\n", status_not_matching);
}

#[test]
fn test_match_header_any_matching() {
    let _m = mock("GET", "/")
        .match_header("Content-Type", Matcher::Any)
        .with_body("matched")
        .create();

    let (_, _, body) = request("GET /", "content-type: something\r\n");
    assert_eq!("matched", body);
}

#[test]
fn test_match_header_any_not_matching() {
    let _m = mock("GET", "/")
        .match_header("Content-Type", Matcher::Any)
        .with_body("matched")
        .create();

    let (status, _, _) = request("GET /", "");
    assert_eq!("HTTP/1.1 501 Mock Not Found\r\n", status);
}

#[test]
fn test_match_header_missing_matching() {
    let _m = mock("GET", "/")
        .match_header("Authorization", Matcher::Missing)
        .create();

    let (status, _, _) = request("GET /", "");
    assert_eq!("HTTP/1.1 200 OK\r\n", status);
}

#[test]
fn test_match_header_missing_not_matching() {
    let _m = mock("GET", "/")
        .match_header("Authorization", Matcher::Missing)
        .create();

    let (status, _, _) = request("GET /", "Authorization: something\r\n");
    assert_eq!("HTTP/1.1 501 Mock Not Found\r\n", status);
}

#[test]
fn test_match_header_missing_not_matching_even_when_empty() {
    let _m = mock("GET", "/")
        .match_header("Authorization", Matcher::Missing)
        .create();

    let (status, _, _) = request("GET /", "Authorization:\r\n");
    assert_eq!("HTTP/1.1 501 Mock Not Found\r\n", status);
}

#[test]
fn test_match_multiple_header_conditions_matching() {
    let _m = mock("GET", "/")
        .match_header("Hello", "World")
        .match_header("Content-Type", Matcher::Any)
        .match_header("Authorization", Matcher::Missing)
        .create();

    let (status, _, _) = request("GET /", "Hello: World\r\nContent-Type: something\r\n");
    assert_eq!("HTTP/1.1 200 OK\r\n", status);
}

#[test]
fn test_match_multiple_header_conditions_not_matching() {
    let _m = mock("GET", "/")
        .match_header("hello", "world")
        .match_header("Content-Type", Matcher::Any)
        .match_header("Authorization", Matcher::Missing)
        .create();

    let (status, _, _) = request("GET /", "Hello: World\r\n");
    assert_eq!("HTTP/1.1 501 Mock Not Found\r\n", status);
}

#[test]
fn test_match_any_body_by_default() {
    let _m = mock("POST", "/").create();

    let (status, _, _) = request_with_body("POST /", "", "hello");
    assert_eq!("HTTP/1.1 200 OK\r\n", status);
}

#[test]
fn test_match_body() {
    let _m = mock("POST", "/").match_body("hello").create();

    let (status, _, _) = request_with_body("POST /", "", "hello");
    assert_eq!("HTTP/1.1 200 OK\r\n", status);
}

#[test]
fn test_match_body_not_matching() {
    let _m = mock("POST", "/").match_body("hello").create();

    let (status, _, _) = request_with_body("POST /", "", "bye");
    assert_eq!("HTTP/1.1 501 Mock Not Found\r\n", status);
}

#[test]
fn test_match_binary_body() {
    let _m = mock("POST", "/")
        .match_body(Path::new("./tests/files/test_payload.bin"))
        .create();

    let mut file_content: Binary = Vec::new();
    fs::File::open("./tests/files/test_payload.bin")
        .unwrap()
        .read_to_end(&mut file_content)
        .unwrap();
    let content_length_header = format!("Content-Length: {}\r\n", file_content.len());
    let (status, _, _) = binary_request("POST /", &content_length_header, file_content);
    assert_eq!("HTTP/1.1 200 OK\r\n", status);
}

#[test]
fn test_does_not_match_binary_body() {
    let _m = mock("POST", "/")
        .match_body(Path::new("./tests/files/test_payload.bin"))
        .create();

    let file_content: Binary = (0..1024).map(|_| rand::random::<u8>()).collect();
    let content_length_header = format!("Content-Length: {}\r\n", file_content.len());
    let (status, _, _) = binary_request("POST /", &content_length_header, file_content);
    assert_eq!("HTTP/1.1 501 Mock Not Found\r\n", status);
}

#[test]
fn test_match_body_with_regex() {
    let _m = mock("POST", "/")
        .match_body(Matcher::Regex("hello".to_string()))
        .create();

    let (status, _, _) = request_with_body("POST /", "", "test hello test");
    assert_eq!("HTTP/1.1 200 OK\r\n", status);
}

#[test]
fn test_match_body_with_regex_not_matching() {
    let _m = mock("POST", "/")
        .match_body(Matcher::Regex("hello".to_string()))
        .create();

    let (status, _, _) = request_with_body("POST /", "", "bye");
    assert_eq!("HTTP/1.1 501 Mock Not Found\r\n", status);
}

#[test]
fn test_match_body_with_json() {
    let _m = mock("POST", "/")
        .match_body(Matcher::Json(json!({"hello":"world", "foo": "bar"})))
        .create();

    let (status, _, _) = request_with_body("POST /", "", r#"{"hello":"world", "foo": "bar"}"#);
    assert_eq!("HTTP/1.1 200 OK\r\n", status);
}

#[test]
fn test_match_body_with_more_headers_with_json() {
    let _m = mock("POST", "/")
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

    let (status, _, _) =
        request_with_body("POST /", &headers, r#"{"hello":"world", "foo": "bar"}"#);
    assert_eq!("HTTP/1.1 200 OK\r\n", status);
}

#[test]
fn test_match_body_with_json_order() {
    let _m = mock("POST", "/")
        .match_body(Matcher::Json(json!({"foo": "bar", "hello": "world"})))
        .create();

    let (status, _, _) = request_with_body("POST /", "", r#"{"hello":"world", "foo": "bar"}"#);
    assert_eq!("HTTP/1.1 200 OK\r\n", status);
}

#[test]
fn test_match_body_with_json_string() {
    let _m = mock("POST", "/")
        .match_body(Matcher::JsonString(
            "{\"hello\":\"world\", \"foo\": \"bar\"}".to_string(),
        ))
        .create();

    let (status, _, _) = request_with_body("POST /", "", r#"{"hello":"world", "foo": "bar"}"#);
    assert_eq!("HTTP/1.1 200 OK\r\n", status);
}

#[test]
fn test_match_body_with_json_string_order() {
    let _m = mock("POST", "/")
        .match_body(Matcher::JsonString(
            "{\"foo\": \"bar\", \"hello\": \"world\"}".to_string(),
        ))
        .create();

    let (status, _, _) = request_with_body("POST /", "", r#"{"hello":"world", "foo": "bar"}"#);
    assert_eq!("HTTP/1.1 200 OK\r\n", status);
}

#[test]
fn test_match_body_with_partial_json() {
    let _m = mock("POST", "/")
        .match_body(Matcher::PartialJson(json!({"hello":"world"})))
        .create();

    let (status, _, _) = request_with_body("POST /", "", r#"{"hello":"world", "foo": "bar"}"#);
    assert_eq!("HTTP/1.1 200 OK\r\n", status);
}

#[test]
fn test_match_body_with_partial_json_and_extra_fields() {
    let _m = mock("POST", "/")
        .match_body(Matcher::PartialJson(json!({"hello":"world", "foo": "bar"})))
        .create();

    let (status, _, _) = request_with_body("POST /", "", r#"{"hello":"world"}"#);
    assert_eq!("HTTP/1.1 501 Mock Not Found\r\n", status);
}

#[test]
fn test_match_body_with_partial_json_string() {
    let _m = mock("POST", "/")
        .match_body(Matcher::PartialJsonString(
            "{\"hello\": \"world\"}".to_string(),
        ))
        .create();

    let (status, _, _) = request_with_body("POST /", "", r#"{"hello":"world", "foo": "bar"}"#);
    assert_eq!("HTTP/1.1 200 OK\r\n", status);
}

#[test]
fn test_match_body_with_partial_json_string_and_extra_fields() {
    let _m = mock("POST", "/")
        .match_body(Matcher::PartialJsonString(
            "{\"foo\": \"bar\", \"hello\": \"world\"}".to_string(),
        ))
        .create();

    let (status, _, _) = request_with_body("POST /", "", r#"{"hello":"world"}"#);
    assert_eq!("HTTP/1.1 501 Mock Not Found\r\n", status);
}

#[test]
fn test_mock_with_status() {
    let _m = mock("GET", "/").with_status(204).with_body("").create();

    let (status_line, _, _) = request("GET /", "");
    assert_eq!("HTTP/1.1 204 No Content\r\n", status_line);
}

#[test]
fn test_mock_with_custom_status() {
    let _m = mock("GET", "/").with_status(333).with_body("").create();

    let (status_line, _, _) = request("GET /", "");
    assert_eq!("HTTP/1.1 333 Custom\r\n", status_line);
}

#[test]
fn test_mock_with_body() {
    let _m = mock("GET", "/").with_body("hello").create();

    let (_, _, body) = request("GET /", "");
    assert_eq!("hello", body);
}

#[test]
fn test_mock_with_fn_body() {
    let _m = mock("GET", "/")
        .with_body_from_fn(|w| {
            w.write_all(b"hel")?;
            w.write_all(b"lo")
        })
        .create();

    let (_, _, body) = request("GET /", "");
    assert_eq!("hello", body);
}

#[test]
fn test_mock_with_header() {
    let _m = mock("GET", "/")
        .with_header("content-type", "application/json")
        .with_body("{}")
        .create();

    let (_, headers, _) = request("GET /", "");
    assert!(headers.contains(&"content-type: application/json".to_string()));
}

#[test]
fn test_mock_with_multiple_headers() {
    let _m = mock("GET", "/")
        .with_header("content-type", "application/json")
        .with_header("x-api-key", "1234")
        .with_body("{}")
        .create();

    let (_, headers, _) = request("GET /", "");
    assert!(headers.contains(&"content-type: application/json".to_string()));
    assert!(headers.contains(&"x-api-key: 1234".to_string()));
}

#[test]
fn test_mock_preserves_header_order() {
    let mut expected_headers = Vec::new();
    let mut mock = mock("GET", "/");

    // Add a large number of headers so getting the same order accidentally is unlikely.
    for i in 0..100 {
        let field = format!("x-custom-header-{}", i);
        let value = "test";
        mock = mock.with_header(&field, value);
        expected_headers.push(format!("{}: {}", field, value));
    }

    let _m = mock.create();

    let (_, headers, _) = request("GET /", "");
    let custom_headers: Vec<_> = headers
        .into_iter()
        .filter(|header| header.starts_with("x-custom-header"))
        .collect();

    assert_eq!(custom_headers, expected_headers);
}

#[test]
fn test_going_out_of_context_removes_mock() {
    {
        let _m = mock("GET", "/reset").create();

        let (working_status_line, _, _) = request("GET /reset", "");
        assert_eq!("HTTP/1.1 200 OK\r\n", working_status_line);
    }

    let (reset_status_line, _, _) = request("GET /reset", "");
    assert_eq!("HTTP/1.1 501 Mock Not Found\r\n", reset_status_line);
}

#[test]
fn test_going_out_of_context_doesnt_remove_other_mocks() {
    let _m1 = mock("GET", "/long").create();

    {
        let _m2 = mock("GET", "/short").create();

        let (short_status_line, _, _) = request("GET /short", "");
        assert_eq!("HTTP/1.1 200 OK\r\n", short_status_line);
    }

    let (long_status_line, _, _) = request("GET /long", "");
    assert_eq!("HTTP/1.1 200 OK\r\n", long_status_line);
}

#[test]
fn test_explicitly_calling_drop_removes_the_mock() {
    let mock = mock("GET", "/").create();

    let (status_line, _, _) = request("GET /", "");
    assert_eq!("HTTP/1.1 200 OK\r\n", status_line);

    mem::drop(mock);

    let (dropped_status_line, _, _) = request("GET /", "");
    assert_eq!("HTTP/1.1 501 Mock Not Found\r\n", dropped_status_line);
}

#[test]
fn test_regex_match_path() {
    let _m1 = mock("GET", Matcher::Regex(r"^/a/\d{1}$".to_string()))
        .with_body("aaa")
        .create();
    let _m2 = mock("GET", Matcher::Regex(r"^/b/\d{1}$".to_string()))
        .with_body("bbb")
        .create();

    let (_, _, body_a) = request("GET /a/1", "");
    assert_eq!("aaa", body_a);

    let (_, _, body_b) = request("GET /b/2", "");
    assert_eq!("bbb", body_b);

    let (status_line, _, _) = request("GET /a/11", "");
    assert_eq!("HTTP/1.1 501 Mock Not Found\r\n", status_line);

    let (status_line, _, _) = request("GET /c/2", "");
    assert_eq!("HTTP/1.1 501 Mock Not Found\r\n", status_line);
}

#[test]
fn test_regex_match_header() {
    let _m = mock("GET", "/")
        .match_header(
            "Authorization",
            Matcher::Regex(r"^Bearer token\.\w+$".to_string()),
        )
        .with_body("{}")
        .create();

    let (_, _, body_json) = request("GET /", "Authorization: Bearer token.payload\r\n");
    assert_eq!("{}", body_json);

    let (status_line, _, _) = request("GET /", "authorization: Beare none\r\n");
    assert_eq!("HTTP/1.1 501 Mock Not Found\r\n", status_line);
}

#[test]
fn test_any_of_match_header() {
    let _m = mock("GET", "/")
        .match_header(
            "Via",
            Matcher::AnyOf(vec![
                Matcher::Exact("one".into()),
                Matcher::Exact("two".into()),
            ]),
        )
        .with_body("{}")
        .create();

    let (_, _, body_json) = request("GET /", "Via: one\r\n");
    assert_eq!("{}", body_json);

    let (_, _, body_json) = request("GET /", "Via: two\r\n");
    assert_eq!("{}", body_json);

    let (_, _, body_json) = request("GET /", "Via: one\r\nVia: two\r\n");
    assert_eq!("{}", body_json);

    let (status_line, _, _) = request("GET /", "Via: one\r\nVia: two\r\nVia: wrong\r\n");
    assert!(status_line.starts_with("HTTP/1.1 501 "));

    let (status_line, _, _) = request("GET /", "Via: wrong\r\n");
    assert!(status_line.starts_with("HTTP/1.1 501 "));
}

#[test]
fn test_any_of_match_body() {
    let _m = mock("GET", "/")
        .match_body(Matcher::AnyOf(vec![
            Matcher::Regex("one".to_string()),
            Matcher::Regex("two".to_string()),
        ]))
        .create();

    let (status_line, _, _) = request_with_body("GET /", "", "one");
    assert!(status_line.starts_with("HTTP/1.1 200 "));

    let (status_line, _, _) = request_with_body("GET /", "", "two");
    assert!(status_line.starts_with("HTTP/1.1 200 "));

    let (status_line, _, _) = request_with_body("GET /", "", "one two");
    assert!(status_line.starts_with("HTTP/1.1 200 "));

    let (status_line, _, _) = request_with_body("GET /", "", "three");
    assert!(status_line.starts_with("HTTP/1.1 501 "));
}

#[test]
fn test_any_of_missing_match_header() {
    let _m = mock("GET", "/")
        .match_header(
            "Via",
            Matcher::AnyOf(vec![Matcher::Exact("one".into()), Matcher::Missing]),
        )
        .with_body("{}")
        .create();

    let (_, _, body_json) = request("GET /", "Via: one\r\n");
    assert_eq!("{}", body_json);

    let (_, _, body_json) = request("GET /", "Via: one\r\nVia: one\r\nVia: one\r\n");
    assert_eq!("{}", body_json);

    let (_, _, body_json) = request("GET /", "NotVia: one\r\n");
    assert_eq!("{}", body_json);

    let (status_line, _, _) = request("GET /", "Via: wrong\r\n");
    assert!(status_line.starts_with("HTTP/1.1 501 "));

    let (status_line, _, _) = request("GET /", "Via: wrong\r\nVia: one\r\n");
    assert!(status_line.starts_with("HTTP/1.1 501 "));

    let (status_line, _, _) = request("GET /", "Via: one\r\nVia: wrong\r\n");
    assert!(status_line.starts_with("HTTP/1.1 501 "));
}

#[test]
fn test_all_of_match_header() {
    let _m = mock("GET", "/")
        .match_header(
            "Via",
            Matcher::AllOf(vec![
                Matcher::Regex("one".into()),
                Matcher::Regex("two".into()),
            ]),
        )
        .with_body("{}")
        .create();

    let (status_line, _, _) = request("GET /", "Via: one\r\n");
    assert!(status_line.starts_with("HTTP/1.1 501 "));

    let (status_line, _, _) = request("GET /", "Via: two\r\n");
    assert!(status_line.starts_with("HTTP/1.1 501 "));

    let (status_line, _, _) = request("GET /", "Via: one two\r\nVia: one two three\r\n");
    assert!(status_line.starts_with("HTTP/1.1 200 "));

    let (status_line, _, _) = request("GET /", "Via: one\r\nVia: two\r\nVia: wrong\r\n");
    assert!(status_line.starts_with("HTTP/1.1 501 "));

    let (status_line, _, _) = request("GET /", "Via: wrong\r\n");
    assert!(status_line.starts_with("HTTP/1.1 501 "));
}

#[test]
fn test_all_of_match_body() {
    let _m = mock("GET", "/")
        .match_body(Matcher::AllOf(vec![
            Matcher::Regex("one".to_string()),
            Matcher::Regex("two".to_string()),
        ]))
        .create();

    let (status_line, _, _) = request_with_body("GET /", "", "one");
    assert!(status_line.starts_with("HTTP/1.1 501 "));

    let (status_line, _, _) = request_with_body("GET /", "", "two");
    assert!(status_line.starts_with("HTTP/1.1 501 "));

    let (status_line, _, _) = request_with_body("GET /", "", "one two");
    assert!(status_line.starts_with("HTTP/1.1 200 "));

    let (status_line, _, _) = request_with_body("GET /", "", "three");
    assert!(status_line.starts_with("HTTP/1.1 501 "));
}

#[test]
fn test_all_of_missing_match_header() {
    let _m = mock("GET", "/")
        .match_header("Via", Matcher::AllOf(vec![Matcher::Missing]))
        .with_body("{}")
        .create();

    let (status_line, _, _) = request("GET /", "Via: one\r\n");
    assert!(status_line.starts_with("HTTP/1.1 501 "));

    let (status_line, _, _) = request("GET /", "Via: one\r\nVia: one\r\nVia: one\r\n");
    assert!(status_line.starts_with("HTTP/1.1 501 "));

    let (status_line, _, _) = request("GET /", "NotVia: one\r\n");
    assert!(status_line.starts_with("HTTP/1.1 200 "));

    let (status_line, _, _) = request("GET /", "Via: wrong\r\n");
    assert!(status_line.starts_with("HTTP/1.1 501 "));

    let (status_line, _, _) = request("GET /", "Via: wrong\r\nVia: one\r\n");
    assert!(status_line.starts_with("HTTP/1.1 501 "));

    let (status_line, _, _) = request("GET /", "Via: one\r\nVia: wrong\r\n");
    assert!(status_line.starts_with("HTTP/1.1 501 "));
}

#[test]
fn test_large_utf8_body() {
    let mock_body: String = rand::thread_rng()
        .sample_iter(&Alphanumeric)
        .map(char::from)
        .take(3 * 1024) // Must be larger than the request read buffer
        .map(char::from)
        .collect();

    let _m = mock("GET", "/").with_body(&mock_body).create();

    let (_, _, body) = request("GET /", "");
    assert_eq!(mock_body, body);
}

#[test]
fn test_body_from_file() {
    let _m = mock("GET", "/")
        .with_body_from_file("tests/files/simple.http")
        .create();
    let (status_line, _, body) = request("GET /", "");
    assert_eq!("HTTP/1.1 200 OK\r\n", status_line);
    assert_eq!("test body\n", body);
}

#[test]
fn test_display_mock_matching_exact_path() {
    let mock = mock("GET", "/hello");

    assert_eq!("\r\nGET /hello\r\n", format!("{}", mock));
}

#[test]
fn test_display_mock_matching_regex_path() {
    let mock = mock("GET", Matcher::Regex(r"^/hello/\d+$".to_string()));

    assert_eq!("\r\nGET ^/hello/\\d+$ (regex)\r\n", format!("{}", mock));
}

#[test]
fn test_display_mock_matching_any_path() {
    let mock = mock("GET", Matcher::Any);

    assert_eq!("\r\nGET (any)\r\n", format!("{}", mock));
}

#[test]
fn test_display_mock_matching_exact_query() {
    let mock = mock("GET", "/test?hello=world");

    assert_eq!("\r\nGET /test?hello=world\r\n", format!("{}", mock));
}

#[test]
fn test_display_mock_matching_regex_query() {
    let mock = mock("GET", "/test").match_query(Matcher::Regex("hello=world".to_string()));

    assert_eq!("\r\nGET /test?hello=world (regex)\r\n", format!("{}", mock));
}

#[test]
fn test_display_mock_matching_any_query() {
    let mock = mock("GET", "/test").match_query(Matcher::Any);

    assert_eq!("\r\nGET /test?(any)\r\n", format!("{}", mock));
}

#[test]
fn test_display_mock_matching_exact_header() {
    let mock = mock("GET", "/")
        .match_header("content-type", "text")
        .create();

    assert_eq!("\r\nGET /\r\ncontent-type: text\r\n", format!("{}", mock));
}

#[test]
fn test_display_mock_matching_multiple_headers() {
    let mock = mock("GET", "/")
        .match_header("content-type", "text")
        .match_header("content-length", Matcher::Regex(r"\d+".to_string()))
        .match_header("authorization", Matcher::Any)
        .match_header("x-request-id", Matcher::Missing)
        .create();

    assert_eq!("\r\nGET /\r\ncontent-type: text\r\ncontent-length: \\d+ (regex)\r\nauthorization: (any)\r\nx-request-id: (missing)\r\n", format!("{}", mock));
}

#[test]
fn test_display_mock_matching_exact_body() {
    let mock = mock("POST", "/").match_body("hello").create();

    assert_eq!("\r\nPOST /\r\nhello\r\n", format!("{}", mock));
}

#[test]
fn test_display_mock_matching_regex_body() {
    let mock = mock("POST", "/")
        .match_body(Matcher::Regex("hello".to_string()))
        .create();

    assert_eq!("\r\nPOST /\r\nhello\r\n", format!("{}", mock));
}

#[test]
fn test_display_mock_matching_any_body() {
    let mock = mock("POST", "/").match_body(Matcher::Any).create();

    assert_eq!("\r\nPOST /\r\n", format!("{}", mock));
}

#[test]
fn test_display_mock_matching_headers_and_body() {
    let mock = mock("POST", "/")
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
    let mock = mock("POST", "/")
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
    let mock = mock("POST", "/")
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
    let mock = mock("GET", "/hello").create();

    request("GET /hello", "");

    mock.assert();
}

#[test]
fn test_expect() {
    let mock = mock("GET", "/hello").expect(3).create();

    request("GET /hello", "");
    request("GET /hello", "");
    request("GET /hello", "");

    mock.assert();
}

#[test]
fn test_expect_at_least_and_at_most() {
    let mock = mock("GET", "/hello")
        .expect_at_least(3)
        .expect_at_most(6)
        .create();

    request("GET /hello", "");
    request("GET /hello", "");
    request("GET /hello", "");
    request("GET /hello", "");
    request("GET /hello", "");

    mock.assert();
}

#[test]
fn test_expect_at_least() {
    let mock = mock("GET", "/hello").expect_at_least(3).create();

    request("GET /hello", "");
    request("GET /hello", "");
    request("GET /hello", "");

    mock.assert();
}

#[test]
fn test_expect_at_least_more() {
    let mock = mock("GET", "/hello").expect_at_least(3).create();

    request("GET /hello", "");
    request("GET /hello", "");
    request("GET /hello", "");
    request("GET /hello", "");

    mock.assert();
}

#[test]
fn test_expect_at_most_with_needed_requests() {
    let mock = mock("GET", "/hello").expect_at_most(3).create();

    request("GET /hello", "");
    request("GET /hello", "");
    request("GET /hello", "");

    mock.assert();
}

#[test]
fn test_expect_at_most_with_few_requests() {
    let mock = mock("GET", "/hello").expect_at_most(3).create();

    request("GET /hello", "");
    request("GET /hello", "");

    mock.assert();
}

#[test]
#[should_panic(
    expected = "\n> Expected at least 3 request(s) to:\n\r\nGET /hello\r\n\n...but received 2\n"
)]
fn test_assert_panics_expect_at_least_with_too_few_requests() {
    let mock = mock("GET", "/hello").expect_at_least(3).create();

    request("GET /hello", "");
    request("GET /hello", "");

    mock.assert();
}

#[test]
#[should_panic(
    expected = "\n> Expected at most 3 request(s) to:\n\r\nGET /hello\r\n\n...but received 4\n"
)]
fn test_assert_panics_expect_at_most_with_too_many_requests() {
    let mock = mock("GET", "/hello").expect_at_most(3).create();

    request("GET /hello", "");
    request("GET /hello", "");
    request("GET /hello", "");
    request("GET /hello", "");

    mock.assert();
}

#[test]
#[should_panic(
    expected = "\n> Expected between 3 and 5 request(s) to:\n\r\nGET /hello\r\n\n...but received 2\n"
)]
fn test_assert_panics_expect_at_least_and_at_most_with_too_few_requests() {
    let mock = mock("GET", "/hello")
        .expect_at_least(3)
        .expect_at_most(5)
        .create();

    request("GET /hello", "");
    request("GET /hello", "");

    mock.assert();
}

#[test]
#[should_panic(
    expected = "\n> Expected between 3 and 5 request(s) to:\n\r\nGET /hello\r\n\n...but received 6\n"
)]
fn test_assert_panics_expect_at_least_and_at_most_with_too_many_requests() {
    let mock = mock("GET", "/hello")
        .expect_at_least(3)
        .expect_at_most(5)
        .create();

    request("GET /hello", "");
    request("GET /hello", "");
    request("GET /hello", "");
    request("GET /hello", "");
    request("GET /hello", "");
    request("GET /hello", "");

    mock.assert();
}

#[test]
#[should_panic(expected = "\n> Expected 1 request(s) to:\n\r\nGET /hello\r\n\n...but received 0\n")]
fn test_assert_panics_if_no_request_was_performed() {
    let mock = mock("GET", "/hello").create();

    mock.assert();
}

#[test]
#[should_panic(expected = "\n> Expected 3 request(s) to:\n\r\nGET /hello\r\n\n...but received 2\n")]
fn test_assert_panics_with_too_few_requests() {
    let mock = mock("GET", "/hello").expect(3).create();

    request("GET /hello", "");
    request("GET /hello", "");

    mock.assert();
}

#[test]
#[should_panic(expected = "\n> Expected 3 request(s) to:\n\r\nGET /hello\r\n\n...but received 4\n")]
fn test_assert_panics_with_too_many_requests() {
    let mock = mock("GET", "/hello").expect(3).create();

    request("GET /hello", "");
    request("GET /hello", "");
    request("GET /hello", "");
    request("GET /hello", "");

    mock.assert();
}

#[test]
#[should_panic(
    expected = "\n> Expected 1 request(s) to:\n\r\nGET /hello\r\n\n...but received 0\n\n> The last unmatched request was:\n\r\nGET /bye\r\n\n> Difference:\n\n\u{1b}[31mGET /hello\n\u{1b}[0m\u{1b}[32mGET\u{1b}[0m\u{1b}[32m \u{1b}[0m\u{1b}[42;37m/bye\u{1b}[0m\u{1b}[32m\n\u{1b}[0m\n\n"
)]
#[cfg(feature = "color")]
fn test_assert_with_last_unmatched_request() {
    let mock = mock("GET", "/hello").create();

    request("GET /bye", "");

    mock.assert();
}

// Same test but without colors (for Appveyor)
#[test]
#[should_panic(
    expected = "\n> Expected 1 request(s) to:\n\r\nGET /hello\r\n\n...but received 0\n\n> The last unmatched request was:\n\r\nGET /bye\r\n\n> Difference:\n\nGET /hello\nGET /bye\n\n\n"
)]
#[cfg(not(feature = "color"))]
fn test_assert_with_last_unmatched_request() {
    let mock = mock("GET", "/hello").create();

    request("GET /bye", "");

    mock.assert();
}

#[test]
#[should_panic(
    expected = "\n> Expected 1 request(s) to:\n\r\nGET /hello\r\n\n...but received 0\n\n> The last unmatched request was:\n\r\nGET /bye\r\nauthorization: 1234\r\naccept: text\r\n\n> Difference:\n\n\u{1b}[31mGET /hello\n\u{1b}[0m\u{1b}[32mGET\u{1b}[0m\u{1b}[32m \u{1b}[0m\u{1b}[42;37m/bye\u{1b}[0m\u{1b}[32m\n\u{1b}[0m\u{1b}[92mauthorization: 1234\n\u{1b}[0m\u{1b}[92maccept: text\n\u{1b}[0m\n\n"
)]
#[cfg(feature = "color")]
fn test_assert_with_last_unmatched_request_and_headers() {
    let mock = mock("GET", "/hello").create();

    request("GET /bye", "authorization: 1234\r\naccept: text\r\n");

    mock.assert();
}

// Same test but without colors (for Appveyor)
#[test]
#[should_panic(
    expected = "\n> Expected 1 request(s) to:\n\r\nGET /hello\r\n\n...but received 0\n\n> The last unmatched request was:\n\r\nGET /bye\r\nauthorization: 1234\r\naccept: text\r\n\n> Difference:\n\nGET /hello\nGET /bye\nauthorization: 1234\naccept: text\n\n\n"
)]
#[cfg(not(feature = "color"))]
fn test_assert_with_last_unmatched_request_and_headers() {
    let mock = mock("GET", "/hello").create();

    request("GET /bye", "authorization: 1234\r\naccept: text\r\n");

    mock.assert();
}

#[test]
#[should_panic(
    expected = "\n> Expected 1 request(s) to:\n\r\nGET /hello\r\n\n...but received 0\n\n> The last unmatched request was:\n\r\nPOST /bye\r\ncontent-length: 5\r\nhello\r\n\n"
)]
fn test_assert_with_last_unmatched_request_and_body() {
    let mock = mock("GET", "/hello").create();

    request_with_body("POST /bye", "", "hello");

    mock.assert();
}

#[test]
fn test_request_from_thread() {
    let mock = mock("GET", "/").create();

    let process = thread::spawn(move || {
        request("GET /", "");
    });

    process.join().unwrap();

    mock.assert();
}

#[test]
#[ignore]
// Can't work unless there's a way to apply LOCAL_TEST_MUTEX only to test threads and
// not to any of their sub-threads.
fn test_mock_from_inside_thread_does_not_lock_forever() {
    let _mock_outside_thread = mock("GET", "/").with_body("outside").create();

    let process = thread::spawn(move || {
        let _mock_inside_thread = mock("GET", "/").with_body("inside").create();
    });

    process.join().unwrap();

    let (_, _, body) = request("GET /", "");

    assert_eq!("outside", body);
}

#[test]
fn test_head_request_with_overridden_content_length() {
    let _mock = mock("HEAD", "/")
        .with_header("content-length", "100")
        .create();

    let (_, headers, _) = request("HEAD /", "");

    assert_eq!(vec!["connection: close", "content-length: 100"], headers);
}

#[test]
fn test_propagate_protocol_to_response() {
    let _mock = mock("GET", "/").create();

    let stream = request_stream("1.0", "GET /", "", "");
    stream.shutdown(Shutdown::Write).unwrap();

    let (status_line, _, _) = parse_stream(stream, true);
    assert_eq!("HTTP/1.0 200 OK\r\n", status_line);
}

#[test]
fn test_large_body_without_content_length() {
    let body = "123".repeat(2048);

    let _mock = mock("POST", "/").match_body(body.as_str()).create();

    let stream = request_stream("1.0", "POST /", "", &body);
    stream.shutdown(Shutdown::Write).unwrap();

    let (status_line, _, _) = parse_stream(stream, false);
    assert_eq!("HTTP/1.0 200 OK\r\n", status_line);
}

#[test]
fn test_transfer_encoding_chunked() {
    let _mock = mock("POST", "/")
        .match_body("Hello, chunked world!")
        .create();

    let body = "3\r\nHel\r\n5\r\nlo, c\r\nD\r\nhunked world!\r\n0\r\n\r\n";

    let (status, _, _) = parse_stream(
        request_stream("1.1", "POST /", "Transfer-Encoding: chunked\r\n", body),
        false,
    );

    assert_eq!("HTTP/1.1 200 OK\r\n", status);
}

#[test]
fn test_match_exact_query() {
    let _m = mock("GET", "/hello")
        .match_query(Matcher::Exact("number=one".to_string()))
        .create();

    let (status_line, _, _) = request("GET /hello?number=one", "");
    assert_eq!("HTTP/1.1 200 OK\r\n", status_line);

    let (status_line, _, _) = request("GET /hello?number=two", "");
    assert_eq!("HTTP/1.1 501 Mock Not Found\r\n", status_line);
}

#[test]
fn test_match_exact_query_via_path() {
    let _m = mock("GET", "/hello?number=one").create();

    let (status_line, _, _) = request("GET /hello?number=one", "");
    assert_eq!("HTTP/1.1 200 OK\r\n", status_line);

    let (status_line, _, _) = request("GET /hello?number=two", "");
    assert_eq!("HTTP/1.1 501 Mock Not Found\r\n", status_line);
}

#[test]
fn test_match_partial_query_by_regex() {
    let _m = mock("GET", "/hello")
        .match_query(Matcher::Regex("number=one".to_string()))
        .create();

    let (status_line, _, _) = request("GET /hello?something=else&number=one", "");
    assert_eq!("HTTP/1.1 200 OK\r\n", status_line);
}

#[test]
fn test_match_partial_query_by_urlencoded() {
    let _m = mock("GET", "/hello")
        .match_query(Matcher::UrlEncoded("num ber".into(), "o ne".into()))
        .create();

    let (status_line, _, _) = request("GET /hello?something=else&num%20ber=o%20ne", "");
    assert_eq!("HTTP/1.1 200 OK\r\n", status_line);

    let (status_line, _, _) = request("GET /hello?something=else&number=one", "");
    assert_eq!("HTTP/1.1 501 Mock Not Found\r\n", status_line);
}

#[test]
fn test_match_partial_query_by_regex_all_of() {
    let _m = mock("GET", "/hello")
        .match_query(Matcher::AllOf(vec![
            Matcher::Regex("number=one".to_string()),
            Matcher::Regex("hello=world".to_string()),
        ]))
        .create();

    let (status_line, _, _) = request("GET /hello?hello=world&something=else&number=one", "");
    assert_eq!("HTTP/1.1 200 OK\r\n", status_line);

    let (status_line, _, _) = request("GET /hello?hello=world&something=else", "");
    assert_eq!("HTTP/1.1 501 Mock Not Found\r\n", status_line);
}

#[test]
fn test_match_partial_query_by_urlencoded_all_of() {
    let _m = mock("GET", "/hello")
        .match_query(Matcher::AllOf(vec![
            Matcher::UrlEncoded("num ber".into(), "o ne".into()),
            Matcher::UrlEncoded("hello".into(), "world".into()),
        ]))
        .create();

    let (status_line, _, _) = request("GET /hello?hello=world&something=else&num%20ber=o%20ne", "");
    assert_eq!("HTTP/1.1 200 OK\r\n", status_line);

    let (status_line, _, _) = request("GET /hello?hello=world&something=else", "");
    assert_eq!("HTTP/1.1 501 Mock Not Found\r\n", status_line);
}

#[test]
fn test_match_query_with_non_percent_url_escaping() {
    let _m = mock("GET", "/hello")
        .match_query(Matcher::AllOf(vec![
            Matcher::UrlEncoded("num ber".into(), "o ne".into()),
            Matcher::UrlEncoded("hello".into(), "world".into()),
        ]))
        .create();

    let (status_line, _, _) = request("GET /hello?hello=world&something=else&num+ber=o+ne", "");
    assert_eq!("HTTP/1.1 200 OK\r\n", status_line);
}

#[test]
fn test_match_missing_query() {
    let _m = mock("GET", "/hello").match_query(Matcher::Missing).create();

    let (status_line, _, _) = request("GET /hello?", "");
    assert_eq!("HTTP/1.1 200 OK\r\n", status_line);

    let (status_line, _, _) = request("GET /hello?number=one", "");
    assert_eq!("HTTP/1.1 501 Mock Not Found\r\n", status_line);
}

#[test]
fn test_anyof_exact_path_and_query_matcher() {
    let mock = mock(
        "GET",
        Matcher::AnyOf(vec![Matcher::Exact("/hello?world".to_string())]),
    )
    .create();

    let (status_line, _, _) = request("GET /hello?world", "");
    assert_eq!("HTTP/1.1 200 OK\r\n", status_line);

    mock.assert();
}

#[test]
fn test_default_headers() {
    let _m = mock("GET", "/").create();

    let (_, headers, _) = request("GET /", "");
    assert_eq!(vec!["connection: close", "content-length: 0"], headers);
}

#[test]
fn test_missing_create_bad() {
    testing_logger::setup();

    let m = mock("GET", "/");
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

    let m = mock("GET", "/").create();
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
    let mock_200 = mock("GET", "/hello").with_status(200).create();
    let mock_404 = mock("GET", "/hello").with_status(404).create();
    let mock_500 = mock("GET", "/hello").with_status(500).create();

    let response_200 = request("GET /hello", "");
    let response_404 = request("GET /hello", "");
    let response_500 = request("GET /hello", "");

    mock_200.assert();
    mock_404.assert();
    mock_500.assert();

    assert_eq!(response_200.0, "HTTP/1.1 200 OK\r\n");
    assert_eq!(response_404.0, "HTTP/1.1 404 Not Found\r\n");
    assert_eq!(response_500.0, "HTTP/1.1 500 Internal Server Error\r\n");
}

#[test]
fn test_same_endpoint_different_responses_last_one_forever() {
    let _mock_200 = mock("GET", "/hello").with_status(200).create();
    let _mock_404 = mock("GET", "/hello").with_status(404).create();
    let _mock_500 = mock("GET", "/hello")
        .expect_at_least(1)
        .with_status(500)
        .create();

    let response_200 = request("GET /hello", "");
    let response_404 = request("GET /hello", "");
    let response_500_1 = request("GET /hello", "");
    let response_500_2 = request("GET /hello", "");
    let response_500_3 = request("GET /hello", "");

    assert_eq!(response_200.0, "HTTP/1.1 200 OK\r\n");
    assert_eq!(response_404.0, "HTTP/1.1 404 Not Found\r\n");
    assert_eq!(response_500_1.0, "HTTP/1.1 500 Internal Server Error\r\n");
    assert_eq!(response_500_2.0, "HTTP/1.1 500 Internal Server Error\r\n");
    assert_eq!(response_500_3.0, "HTTP/1.1 500 Internal Server Error\r\n");
}

#[test]
fn test_matched_bool() {
    let m = mock("GET", "/").create();

    let (_, _, _) = request_with_body("GET /", "", "");
    m.assert();
    assert!(m.matched(), "matched method returns correctly");

    let (_, _, _) = request_with_body("GET /", "", "");
    assert!(!m.matched(), "matched method returns correctly");
}

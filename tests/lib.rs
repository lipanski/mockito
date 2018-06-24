extern crate rand;
extern crate mockito;
#[macro_use] extern crate serde_json;

use std::net::TcpStream;
use std::io::{Read, Write, BufRead, BufReader};
use std::str::FromStr;
use std::mem;
use std::thread;
use rand::Rng;
use mockito::{SERVER_ADDRESS, mock, Matcher};

fn request_stream(route: &str, headers: &str, body: &str) -> TcpStream {
    let mut stream = TcpStream::connect(SERVER_ADDRESS).unwrap();
    let message = [route, " HTTP/1.1\r\n", headers, "\r\n", body].join("");
    stream.write_all(message.as_bytes()).unwrap();

    stream
}

fn parse_stream(stream: TcpStream) -> (String, Vec<String>, String) {
    let mut reader = BufReader::new(stream);

    let mut status_line = String::new();
    reader.read_line(&mut status_line).unwrap();

    let mut headers = vec![];
    let mut content_length: u64 = 0;
    loop {
        let mut header_line = String::new();
        reader.read_line(&mut header_line).unwrap();

        if header_line == "\r\n" { break }

        if header_line.starts_with("content-length:") {
            let mut parts = header_line.split(":");
            content_length = u64::from_str(parts.nth(1).unwrap().trim()).unwrap();
        }

        headers.push(header_line.trim_right().to_string());
    }

    let mut body = String::new();
    reader.take(content_length).read_to_string(&mut body).unwrap();

    (status_line, headers, body)
}

fn request(route: &str, headers: &str) -> (String, Vec<String>, String) {
    parse_stream(request_stream(route, headers, ""))
}

fn request_with_body(route: &str, headers: &str, body: &str) -> (String, Vec<String>, String) {
    let headers = format!("{}content-length: {}\r\n", headers, body.len());
    parse_stream(request_stream(route, &headers, body))
}

#[test]
fn test_create_starts_the_server() {
    let _m = mock("GET", "/").with_body("hello").create();

    let stream = TcpStream::connect(SERVER_ADDRESS);
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

    let (status_line, _, _) = request("GET /nope", "");
    assert_eq!("HTTP/1.1 501 Not Implemented\r\n", status_line);
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
    let _m = mock("GET", "/").match_header("content-type", "text/plain").create();

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

    let (_, _, body_matching) = request("GET /", "content-type: text/plain\r\nauthorization: secret\r\n");
    assert_eq!("matched", body_matching);

    let (status_not_matching, _, _) = request("GET /", "content-type: text/plain\r\nauthorization: meh\r\n");
    assert_eq!("HTTP/1.1 501 Not Implemented\r\n", status_not_matching);
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
    assert_eq!("HTTP/1.1 501 Not Implemented\r\n", status);
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
    assert_eq!("HTTP/1.1 501 Not Implemented\r\n", status);
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
    assert_eq!("HTTP/1.1 501 Not Implemented\r\n", status);
}

#[test]
fn test_match_any_body_by_default() {
    let _m = mock("POST", "/").create();

    let (status, _, _) = request_with_body("POST /", "", "hello");
    assert_eq!("HTTP/1.1 200 OK\r\n", status);
}

#[test]
fn test_match_body() {
    let _m = mock("POST", "/")
        .match_body("hello")
        .create();

    let (status, _, _) = request_with_body("POST /", "", "hello");
    assert_eq!("HTTP/1.1 200 OK\r\n", status);
}

#[test]
fn test_match_body_not_matching() {
    let _m = mock("POST", "/")
        .match_body("hello")
        .create();

    let (status, _, _) = request_with_body("POST /", "", "bye");
    assert_eq!("HTTP/1.1 501 Not Implemented\r\n", status);
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
    assert_eq!("HTTP/1.1 501 Not Implemented\r\n", status);
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
       .match_body(Matcher::JsonString("{\"hello\":\"world\", \"foo\": \"bar\"}".to_string()))
       .create();

   let (status, _, _) = request_with_body("POST /", "", r#"{"hello":"world", "foo": "bar"}"#);
   assert_eq!("HTTP/1.1 200 OK\r\n", status);
}

#[test]
fn test_match_body_with_json_string_order() {
    let _m = mock("POST", "/")
        .match_body(Matcher::JsonString("{\"foo\": \"bar\", \"hello\": \"world\"}".to_string()))
        .create();

    let (status, _, _) = request_with_body("POST /", "", r#"{"hello":"world", "foo": "bar"}"#);
    assert_eq!("HTTP/1.1 200 OK\r\n", status);
}


#[test]
fn test_mock_with_status() {
    let _m = mock("GET", "/")
        .with_status(204)
        .with_body("")
        .create();

    let (status_line, _, _) = request("GET /", "");
    assert_eq!("HTTP/1.1 204 No Content\r\n", status_line);
}

#[test]
fn test_mock_with_custom_status() {
    let _m = mock("GET", "/")
        .with_status(333)
        .with_body("")
        .create();

    let (status_line, _, _) = request("GET /", "");
    assert_eq!("HTTP/1.1 333 Custom\r\n", status_line);
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
    let custom_headers: Vec<_> = headers.into_iter()
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
    assert_eq!("HTTP/1.1 501 Not Implemented\r\n", reset_status_line);
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
    assert_eq!("HTTP/1.1 501 Not Implemented\r\n", dropped_status_line);
}

#[test]
fn test_regex_match_path() {
    let _m1 = mock("GET", Matcher::Regex(r"^/a/\d{1}$".to_string())).with_body("aaa").create();
    let _m2 = mock("GET", Matcher::Regex(r"^/b/\d{1}$".to_string())).with_body("bbb").create();

    let (_, _, body_a) = request("GET /a/1", "");
    assert_eq!("aaa", body_a);

    let (_, _, body_b) = request("GET /b/2", "");
    assert_eq!("bbb", body_b);

    let (status_line, _, _) = request("GET /a/11", "");
    assert_eq!("HTTP/1.1 501 Not Implemented\r\n", status_line);

    let (status_line, _, _) = request("GET /c/2", "");
    assert_eq!("HTTP/1.1 501 Not Implemented\r\n", status_line);
}

#[test]
fn test_regex_match_header() {
    let _m = mock("GET", "/")
        .match_header("Authorization", Matcher::Regex(r"^Bearer token\.\w+$".to_string()))
        .with_body("{}")
        .create();

    let (_, _, body_json) = request("GET /", "Authorization: Bearer token.payload\r\n");
    assert_eq!("{}", body_json);

    let (status_line, _, _) = request("GET /", "authorization: Beare none\r\n");
    assert_eq!("HTTP/1.1 501 Not Implemented\r\n", status_line);
}

#[test]
fn test_large_utf8_body() {
    let mock_body: String = rand::thread_rng()
        .gen_iter::<char>()
        .take(3 * 1024) // Must be larger than the request read buffer
        .collect();

    let _m = mock("GET", "/").with_body(&mock_body).create();

    let (_, _, body) = request("GET /", "");
    assert_eq!(mock_body, body);
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
fn test_display_mock_matching_exact_header() {
    let mock = mock("GET", "/").match_header("content-type", "text").create();

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
    let mock = mock("POST", "/").match_body(Matcher::Regex("hello".to_string())).create();

    assert_eq!("\r\nPOST /\r\nhello\r\n", format!("{}", mock));
}

#[test]
fn test_display_mock_matching_any_body() {
    let mock = mock("POST", "/").match_body(Matcher::Any).create();

    assert_eq!("\r\nPOST /\r\n", format!("{}", mock));
}

#[test]
fn test_display_mock_matching_headers_and_body() {
    let mock = mock("POST", "/").match_header("content-type", "text").match_body("hello").create();

    assert_eq!("\r\nPOST /\r\ncontent-type: text\r\nhello\r\n", format!("{}", mock));
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
#[should_panic(expected = "Expected 1 request(s) to:\r\n\r\nGET /hello\r\n\r\n...but received 0\r\n")]
fn test_assert_panics_if_no_request_was_performed() {
    let mock = mock("GET", "/hello").create();

    mock.assert();
}

#[test]
#[should_panic(expected = "Expected 3 request(s) to:\r\n\r\nGET /hello\r\n\r\n...but received 2\r\n")]
fn test_assert_panics_with_too_few_requests() {
    let mock = mock("GET", "/hello").expect(3).create();

    request("GET /hello", "");
    request("GET /hello", "");

    mock.assert();
}

#[test]
#[should_panic(expected = "Expected 3 request(s) to:\r\n\r\nGET /hello\r\n\r\n...but received 4\r\n")]
fn test_assert_panics_with_too_many_requests() {
    let mock = mock("GET", "/hello").expect(3).create();

    request("GET /hello", "");
    request("GET /hello", "");
    request("GET /hello", "");
    request("GET /hello", "");

    mock.assert();
}

#[test]
#[should_panic(expected = "Expected 1 request(s) to:\r\n\r\nGET /hello\r\n\r\n...but received 0\r\n\r\nThe last unmatched request was:\r\n\r\nGET /bye\r\n\r\n")]
fn test_assert_with_last_unmatched_request() {
    let mock = mock("GET", "/hello").create();

    request("GET /bye", "");

    mock.assert();
}

#[test]
#[should_panic(expected = "Expected 1 request(s) to:\r\n\r\nGET /hello\r\n\r\n...but received 0\r\n\r\nThe last unmatched request was:\r\n\r\nGET /bye\r\nauthorization: 1234\r\naccept: text\r\n\r\n")]
fn test_assert_with_last_unmatched_request_and_headers() {
    let mock = mock("GET", "/hello").create();

    request("GET /bye", "authorization: 1234\r\naccept: text\r\n");

    mock.assert();
}

#[test]
#[should_panic(expected = "Expected 1 request(s) to:\r\n\r\nGET /hello\r\n\r\n...but received 0\r\n\r\nThe last unmatched request was:\r\n\r\nPOST /bye\r\ncontent-length: 5\r\nhello\r\n\r\n")]
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
fn test_mock_from_inside_thread_does_not_lock_forever() {
    let _mock_outside_thread = mock("GET", "/").with_body("outside").create();

    let process = thread::spawn(move || {
        let _mock_inside_thread = mock("GET", "/").with_body("inside").create();
    });

    process.join().unwrap();

    let (_, _, body) = request("GET /", "");

    assert_eq!("outside", body);
}

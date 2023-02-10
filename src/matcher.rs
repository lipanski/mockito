use assert_json_diff::{assert_json_matches_no_panic, CompareMode};
use regex::Regex;
use std::collections::HashMap;
use std::convert::From;
use std::fmt;
use std::fs::File;
use std::io;
use std::io::Read;
use std::path::Path;
use std::string::ToString;

///
/// Allows matching the request path, headers or body in multiple ways: by the exact value, by any value (as
/// long as it is present), by regular expression or by checking that a particular header is missing.
///
/// These matchers can be used within the `Server::mock`, `Mock::match_header` or `Mock::match_body` calls.
///
#[derive(Clone, PartialEq, Debug)]
#[allow(deprecated)] // Rust bug #38832
pub enum Matcher {
    /// Matches the exact path or header value. There's also an implementation of `From<&str>`
    /// to keep things simple and backwards compatible.
    Exact(String),
    /// Matches the body content as a binary file
    Binary(BinaryBody),
    /// Matches a path or header value by a regular expression.
    Regex(String),
    /// Matches a specified JSON body from a `serde_json::Value`
    Json(serde_json::Value),
    /// Matches a specified JSON body from a `String`
    JsonString(String),
    /// Matches a partial JSON body from a `serde_json::Value`
    PartialJson(serde_json::Value),
    /// Matches a specified partial JSON body from a `String`
    PartialJsonString(String),
    /// Matches a URL-encoded key/value pair, where both key and value should be specified
    /// in plain (unencoded) format
    UrlEncoded(String, String),
    /// At least one matcher must match
    AnyOf(Vec<Matcher>),
    /// All matchers must match
    AllOf(Vec<Matcher>),
    /// Matches any path or any header value.
    Any,
    /// Checks that a header is not present in the request.
    Missing,
}

impl<'a> From<&'a str> for Matcher {
    fn from(value: &str) -> Self {
        Matcher::Exact(value.to_string())
    }
}

#[allow(clippy::fallible_impl_from)]
impl From<&Path> for Matcher {
    fn from(value: &Path) -> Self {
        // We want the code to panic if the path is not readable.
        Matcher::Binary(BinaryBody::from_path(value).unwrap())
    }
}

impl From<&mut File> for Matcher {
    fn from(value: &mut File) -> Self {
        Matcher::Binary(BinaryBody::from_file(value))
    }
}

impl From<Vec<u8>> for Matcher {
    fn from(value: Vec<u8>) -> Self {
        Matcher::Binary(BinaryBody::from_bytes(value))
    }
}

impl fmt::Display for Matcher {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let join_matches = |matches: &[Self]| {
            matches
                .iter()
                .map(Self::to_string)
                .fold(String::new(), |acc, matcher| {
                    if acc.is_empty() {
                        matcher
                    } else {
                        format!("{}, {}", acc, matcher)
                    }
                })
        };

        let result = match self {
            Matcher::Exact(ref value) => value.to_string(),
            Matcher::Binary(ref file) => format!("{} (binary)", file),
            Matcher::Regex(ref value) => format!("{} (regex)", value),
            Matcher::Json(ref json_obj) => format!("{} (json)", json_obj),
            Matcher::JsonString(ref value) => format!("{} (json)", value),
            Matcher::PartialJson(ref json_obj) => format!("{} (partial json)", json_obj),
            Matcher::PartialJsonString(ref value) => format!("{} (partial json)", value),
            Matcher::UrlEncoded(ref field, ref value) => {
                format!("{}={} (urlencoded)", field, value)
            }
            Matcher::Any => "(any)".to_string(),
            Matcher::AnyOf(x) => format!("({}) (any of)", join_matches(x)),
            Matcher::AllOf(x) => format!("({}) (all of)", join_matches(x)),
            Matcher::Missing => "(missing)".to_string(),
        };
        write!(f, "{}", result)
    }
}

impl Matcher {
    pub(crate) fn matches_values(&self, header_values: &[&str]) -> bool {
        match self {
            Matcher::Missing => header_values.is_empty(),
            // AnyOf([…Missing…]) is handled here, but
            // AnyOf([Something]) is handled in the last block.
            // That's because Missing matches against all values at once,
            // but other matchers match against individual values.
            Matcher::AnyOf(ref matchers) if header_values.is_empty() => {
                matchers.iter().any(|m| m.matches_values(header_values))
            }
            Matcher::AllOf(ref matchers) if header_values.is_empty() => {
                matchers.iter().all(|m| m.matches_values(header_values))
            }
            _ => {
                !header_values.is_empty() && header_values.iter().all(|val| self.matches_value(val))
            }
        }
    }

    pub(crate) fn matches_binary_value(&self, binary: &[u8]) -> bool {
        match self {
            Matcher::Binary(ref file) => binary == &*file.content,
            _ => false,
        }
    }

    #[allow(deprecated)]
    pub(crate) fn matches_value(&self, other: &str) -> bool {
        let compare_json_config = assert_json_diff::Config::new(CompareMode::Inclusive);
        match self {
            Matcher::Exact(ref value) => value == other,
            Matcher::Binary(_) => false,
            Matcher::Regex(ref regex) => Regex::new(regex).unwrap().is_match(other),
            Matcher::Json(ref json_obj) => {
                let other: serde_json::Value = serde_json::from_str(other).unwrap();
                *json_obj == other
            }
            Matcher::JsonString(ref value) => {
                let value: serde_json::Value = serde_json::from_str(value).unwrap();
                let other: serde_json::Value = serde_json::from_str(other).unwrap();
                value == other
            }
            Matcher::PartialJson(ref json_obj) => {
                let actual: serde_json::Value = serde_json::from_str(other).unwrap();
                let expected = json_obj.clone();
                assert_json_matches_no_panic(&actual, &expected, compare_json_config).is_ok()
            }
            Matcher::PartialJsonString(ref value) => {
                let expected: serde_json::Value = serde_json::from_str(value).unwrap();
                let actual: serde_json::Value = serde_json::from_str(other).unwrap();
                assert_json_matches_no_panic(&actual, &expected, compare_json_config).is_ok()
            }
            Matcher::UrlEncoded(ref expected_field, ref expected_value) => {
                serde_urlencoded::from_str::<HashMap<String, String>>(other)
                    .map(|params: HashMap<_, _>| {
                        params.into_iter().any(|(ref field, ref value)| {
                            field == expected_field && value == expected_value
                        })
                    })
                    .unwrap_or(false)
            }
            Matcher::Any => true,
            Matcher::AnyOf(ref matchers) => matchers.iter().any(|m| m.matches_value(other)),
            Matcher::AllOf(ref matchers) => matchers.iter().all(|m| m.matches_value(other)),
            Matcher::Missing => other.is_empty(),
        }
    }
}

#[derive(Clone, PartialEq, Debug)]
pub(crate) enum PathAndQueryMatcher {
    Unified(Matcher),
    Split(Box<Matcher>, Box<Matcher>),
}

impl PathAndQueryMatcher {
    pub(crate) fn matches_value(&self, other: &str) -> bool {
        match self {
            PathAndQueryMatcher::Unified(matcher) => matcher.matches_value(other),
            PathAndQueryMatcher::Split(ref path_matcher, ref query_matcher) => {
                let mut parts = other.splitn(2, '?');
                let path = parts.next().unwrap();
                let query = parts.next().unwrap_or("");

                path_matcher.matches_value(path) && query_matcher.matches_value(query)
            }
        }
    }
}

impl fmt::Display for PathAndQueryMatcher {
    #[allow(deprecated)]
    #[allow(clippy::write_with_newline)]
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            PathAndQueryMatcher::Unified(matcher) => write!(f, "{}\r\n", &matcher),
            PathAndQueryMatcher::Split(path, query) => write!(f, "{}?{}\r\n", &path, &query),
        }
    }
}

///
/// Represents a binary object the body should be matched against
///
#[derive(Debug, Clone)]
pub struct BinaryBody {
    path: Option<String>,
    content: Vec<u8>,
}

impl BinaryBody {
    /// Read the content from path and initialize a `BinaryBody`
    ///
    /// # Errors
    ///
    /// The same resulting from a failed `std::fs::read`.
    pub fn from_path(path: &Path) -> Result<Self, io::Error> {
        Ok(Self {
            path: path.to_str().map(ToString::to_string),
            content: std::fs::read(path)?,
        })
    }

    /// Read the content from a &mut File and initialize a `BinaryBody`
    pub fn from_file(file: &mut File) -> Self {
        Self {
            path: None,
            content: get_content_from(file),
        }
    }

    /// Instantiate the matcher directly passing the content
    #[allow(clippy::missing_const_for_fn)]
    pub fn from_bytes(content: Vec<u8>) -> Self {
        Self {
            path: None,
            content,
        }
    }
}

fn get_content_from(file: &mut File) -> Vec<u8> {
    let mut filecontent: Vec<u8> = Vec::new();
    file.read_to_end(&mut filecontent).unwrap();
    filecontent
}

impl PartialEq for BinaryBody {
    fn eq(&self, other: &Self) -> bool {
        match (self.path.as_ref(), other.path.as_ref()) {
            (Some(p), Some(o)) => p == o,
            _ => self.content == other.content,
        }
    }
}

impl fmt::Display for BinaryBody {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        if let Some(filepath) = self.path.as_ref() {
            write!(f, "filepath: {}", filepath)
        } else {
            let len: usize = std::cmp::min(self.content.len(), 8);
            let first_bytes: Vec<u8> = self.content.iter().copied().take(len).collect();
            write!(f, "filecontent: {:?}", first_bytes)
        }
    }
}

use std::convert::From;
use std::fmt;
use std::io;
use std::sync::Arc;

#[derive(Clone, Debug, PartialEq)]
pub struct Response {
    pub status: Status,
    pub headers: Vec<(String, String)>,
    pub body: Body,
}

type BodyFn = dyn Fn(&mut dyn io::Write) -> io::Result<()> + Send + Sync + 'static;

#[derive(Clone)]
pub enum Body {
    Bytes(Vec<u8>),
    Fn(Arc<BodyFn>),
}

impl fmt::Debug for Body {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match *self {
            Self::Bytes(ref b) => b.fmt(f),
            Self::Fn(_) => f.write_str("<callback>"),
        }
    }
}

impl PartialEq for Body {
    fn eq(&self, other: &Self) -> bool {
        match (self, other) {
            (Self::Bytes(ref a), Self::Bytes(ref b)) => a == b,
            (Self::Fn(ref a), Self::Fn(ref b)) => std::ptr::eq(
                a.as_ref() as *const BodyFn as *const u8,
                b.as_ref() as *const BodyFn as *const u8,
            ),
            _ => false,
        }
    }
}

impl Default for Response {
    fn default() -> Self {
        Self {
            status: Status::Ok,
            headers: vec![("connection".into(), "close".into())],
            body: Body::Bytes(Vec::new()),
        }
    }
}

pub struct Chunked<W: io::Write> {
    writer: W,
}

impl<W: io::Write> Chunked<W> {
    pub fn new(writer: W) -> Self {
        Self { writer }
    }

    pub fn finish(mut self) -> io::Result<W> {
        self.writer.write_all(b"0\r\n\r\n")?;
        Ok(self.writer)
    }
}

impl<W: io::Write> io::Write for Chunked<W> {
    fn write(&mut self, buf: &[u8]) -> io::Result<usize> {
        if buf.is_empty() {
            return Ok(0);
        }
        self.writer
            .write_all(format!("{:x}\r\n", buf.len()).as_bytes())?;
        self.writer.write_all(buf)?;
        self.writer.write_all(b"\r\n")?;
        Ok(buf.len())
    }

    fn flush(&mut self) -> io::Result<()> {
        self.writer.flush()
    }
}

#[derive(Clone, Debug, PartialEq)]
#[cfg_attr(feature = "cargo-clippy", allow(clippy::pub_enum_variant_names))]
pub enum Status {
    Continue,
    SwitchingProtocols,
    Processing,
    Ok,
    Created,
    Accepted,
    NonAuthoritativeInformation,
    NoContent,
    ResetContent,
    PartialContent,
    MultiStatus,
    AlreadyReported,
    IMUsed,
    MultipleChoices,
    MovedPermanently,
    Found,
    SeeOther,
    NotModified,
    UseProxy,
    TemporaryRedirect,
    PermanentRedirect,
    BadRequest,
    Unauthorized,
    PaymentRequired,
    Forbidden,
    NotFound,
    MethodNotAllowed,
    NotAcceptable,
    ProxyAuthenticationRequired,
    RequestTimeout,
    Conflict,
    Gone,
    LengthRequired,
    PreconditionFailed,
    PayloadTooLarge,
    RequestURITooLong,
    UnsupportedMediaType,
    RequestedRangeNotSatisfiable,
    ExpectationFailed,
    ImATeapot,
    MisdirectedRequest,
    UnprocessableEntity,
    Locked,
    FailedDependency,
    UpgradeRequired,
    PreconditionRequired,
    TooManyRequests,
    RequestHeaderFieldsTooLarge,
    ConnectionClosedWithoutResponse,
    UnavailableForLegalReasons,
    ClientClosedRequest,
    InternalServerError,
    NotImplemented,
    BadGateway,
    ServiceUnavailable,
    GatewayTimeout,
    HTTPVersionNotSupported,
    VariantAlsoNegotiates,
    InsufficientStorage,
    LoopDetected,
    NotExtended,
    NetworkAuthenticationRequired,
    NetworkConnectTimeoutError,
    Custom(String),
}

impl From<usize> for Status {
    fn from(status_code: usize) -> Self {
        match status_code {
            100 => Self::Continue,
            101 => Self::SwitchingProtocols,
            102 => Self::Processing,
            200 => Self::Ok,
            201 => Self::Created,
            202 => Self::Accepted,
            203 => Self::NonAuthoritativeInformation,
            204 => Self::NoContent,
            205 => Self::ResetContent,
            206 => Self::PartialContent,
            207 => Self::MultiStatus,
            208 => Self::AlreadyReported,
            226 => Self::IMUsed,
            300 => Self::MultipleChoices,
            301 => Self::MovedPermanently,
            302 => Self::Found,
            303 => Self::SeeOther,
            304 => Self::NotModified,
            305 => Self::UseProxy,
            307 => Self::TemporaryRedirect,
            308 => Self::PermanentRedirect,
            400 => Self::BadRequest,
            401 => Self::Unauthorized,
            402 => Self::PaymentRequired,
            403 => Self::Forbidden,
            404 => Self::NotFound,
            405 => Self::MethodNotAllowed,
            406 => Self::NotAcceptable,
            407 => Self::ProxyAuthenticationRequired,
            408 => Self::RequestTimeout,
            409 => Self::Conflict,
            410 => Self::Gone,
            411 => Self::LengthRequired,
            412 => Self::PreconditionFailed,
            413 => Self::PayloadTooLarge,
            414 => Self::RequestURITooLong,
            415 => Self::UnsupportedMediaType,
            416 => Self::RequestedRangeNotSatisfiable,
            417 => Self::ExpectationFailed,
            418 => Self::ImATeapot,
            421 => Self::MisdirectedRequest,
            422 => Self::UnprocessableEntity,
            423 => Self::Locked,
            424 => Self::FailedDependency,
            426 => Self::UpgradeRequired,
            428 => Self::PreconditionRequired,
            429 => Self::TooManyRequests,
            431 => Self::RequestHeaderFieldsTooLarge,
            444 => Self::ConnectionClosedWithoutResponse,
            451 => Self::UnavailableForLegalReasons,
            499 => Self::ClientClosedRequest,
            500 => Self::InternalServerError,
            501 => Self::NotImplemented,
            502 => Self::BadGateway,
            503 => Self::ServiceUnavailable,
            504 => Self::GatewayTimeout,
            505 => Self::HTTPVersionNotSupported,
            506 => Self::VariantAlsoNegotiates,
            507 => Self::InsufficientStorage,
            508 => Self::LoopDetected,
            510 => Self::NotExtended,
            511 => Self::NetworkAuthenticationRequired,
            599 => Self::NetworkConnectTimeoutError,
            _ => Self::Custom(format!("{} Custom", status_code)),
        }
    }
}

impl fmt::Display for Status {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let formatted = match self {
            Self::Continue => "100 Continue",
            Self::SwitchingProtocols => "101 Switching Protocols",
            Self::Processing => "102 Processing",
            Self::Ok => "200 OK",
            Self::Created => "201 Created",
            Self::Accepted => "202 Accepted",
            Self::NonAuthoritativeInformation => "203 Non-Authoritative Information",
            Self::NoContent => "204 No Content",
            Self::ResetContent => "205 Reset Content",
            Self::PartialContent => "206 Partial Content",
            Self::MultiStatus => "207 Multi-Status",
            Self::AlreadyReported => "208 Already Reported",
            Self::IMUsed => "226 IM Used",
            Self::MultipleChoices => "300 Multiple Choices",
            Self::MovedPermanently => "301 Moved Permanently",
            Self::Found => "302 Found",
            Self::SeeOther => "303 See Other",
            Self::NotModified => "304 Not Modified",
            Self::UseProxy => "305 Use Proxy",
            Self::TemporaryRedirect => "307 Temporary Redirect",
            Self::PermanentRedirect => "308 Permanent Redirect",
            Self::BadRequest => "400 Bad Request",
            Self::Unauthorized => "401 Unauthorized",
            Self::PaymentRequired => "402 Payment Required",
            Self::Forbidden => "403 Forbidden",
            Self::NotFound => "404 Not Found",
            Self::MethodNotAllowed => "405 Method Not Allowed",
            Self::NotAcceptable => "406 Not Acceptable",
            Self::ProxyAuthenticationRequired => "407 Proxy Authentication Required",
            Self::RequestTimeout => "408 Request Timeout",
            Self::Conflict => "409 Conflict",
            Self::Gone => "410 Gone",
            Self::LengthRequired => "411 Length Required",
            Self::PreconditionFailed => "412 Precondition Failed",
            Self::PayloadTooLarge => "413 Payload Too Large",
            Self::RequestURITooLong => "414 Request-URI Too Long",
            Self::UnsupportedMediaType => "415 Unsupported Media Type",
            Self::RequestedRangeNotSatisfiable => "416 Requested Range Not Satisfiable",
            Self::ExpectationFailed => "417 Expectation Failed",
            Self::ImATeapot => "418 I'm a teapot",
            Self::MisdirectedRequest => "421 Misdirected Request",
            Self::UnprocessableEntity => "422 Unprocessable Entity",
            Self::Locked => "423 Locked",
            Self::FailedDependency => "424 Failed Dependency",
            Self::UpgradeRequired => "426 Upgrade Required",
            Self::PreconditionRequired => "428 Precondition Required",
            Self::TooManyRequests => "429 Too Many Requests",
            Self::RequestHeaderFieldsTooLarge => "431 Request Header Fields Too Large",
            Self::ConnectionClosedWithoutResponse => "444 Connection Closed Without Response",
            Self::UnavailableForLegalReasons => "451 Unavailable For Legal Reasons",
            Self::ClientClosedRequest => "499 Client Closed Request",
            Self::InternalServerError => "500 Internal Server Error",
            Self::NotImplemented => "501 Not Implemented",
            Self::BadGateway => "502 Bad Gateway",
            Self::ServiceUnavailable => "503 Service Unavailable",
            Self::GatewayTimeout => "504 Gateway Timeout",
            Self::HTTPVersionNotSupported => "505 HTTP Version Not Supported",
            Self::VariantAlsoNegotiates => "506 Variant Also Negotiates",
            Self::InsufficientStorage => "507 Insufficient Storage",
            Self::LoopDetected => "508 Loop Detected",
            Self::NotExtended => "510 Not Extended",
            Self::NetworkAuthenticationRequired => "511 Network Authentication Required",
            Self::NetworkConnectTimeoutError => "599 Network Connect Timeout Error",
            Self::Custom(ref status_code) => status_code,
        };

        write!(f, "{}", formatted)
    }
}

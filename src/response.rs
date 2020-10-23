use std::convert::From;
use std::fmt;
use std::io;
use std::sync::Arc;

#[derive(Clone, Debug, PartialEq)]
pub(crate) struct Response {
    pub status: Status,
    pub headers: Vec<(String, String)>,
    pub body: Body,
}

type BodyFn = dyn Fn(&mut dyn io::Write) -> io::Result<()> + Send + Sync + 'static;

#[derive(Clone)]
pub(crate) enum Body {
    Bytes(Vec<u8>),
    Fn(Arc<BodyFn>),
}

impl fmt::Debug for Body {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match *self {
            Body::Bytes(ref b) => b.fmt(f),
            Body::Fn(_) => f.write_str("<callback>"),
        }
    }
}

impl PartialEq for Body {
    fn eq(&self, other: &Self) -> bool {
        match (self, other) {
            (Body::Bytes(ref a), Body::Bytes(ref b)) => a == b,
            (Body::Fn(ref a), Body::Fn(ref b)) => std::ptr::eq(
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

pub(crate) struct Chunked<W: io::Write> {
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
            100 => Status::Continue,
            101 => Status::SwitchingProtocols,
            102 => Status::Processing,
            200 => Status::Ok,
            201 => Status::Created,
            202 => Status::Accepted,
            203 => Status::NonAuthoritativeInformation,
            204 => Status::NoContent,
            205 => Status::ResetContent,
            206 => Status::PartialContent,
            207 => Status::MultiStatus,
            208 => Status::AlreadyReported,
            226 => Status::IMUsed,
            300 => Status::MultipleChoices,
            301 => Status::MovedPermanently,
            302 => Status::Found,
            303 => Status::SeeOther,
            304 => Status::NotModified,
            305 => Status::UseProxy,
            307 => Status::TemporaryRedirect,
            308 => Status::PermanentRedirect,
            400 => Status::BadRequest,
            401 => Status::Unauthorized,
            402 => Status::PaymentRequired,
            403 => Status::Forbidden,
            404 => Status::NotFound,
            405 => Status::MethodNotAllowed,
            406 => Status::NotAcceptable,
            407 => Status::ProxyAuthenticationRequired,
            408 => Status::RequestTimeout,
            409 => Status::Conflict,
            410 => Status::Gone,
            411 => Status::LengthRequired,
            412 => Status::PreconditionFailed,
            413 => Status::PayloadTooLarge,
            414 => Status::RequestURITooLong,
            415 => Status::UnsupportedMediaType,
            416 => Status::RequestedRangeNotSatisfiable,
            417 => Status::ExpectationFailed,
            418 => Status::ImATeapot,
            421 => Status::MisdirectedRequest,
            422 => Status::UnprocessableEntity,
            423 => Status::Locked,
            424 => Status::FailedDependency,
            426 => Status::UpgradeRequired,
            428 => Status::PreconditionRequired,
            429 => Status::TooManyRequests,
            431 => Status::RequestHeaderFieldsTooLarge,
            444 => Status::ConnectionClosedWithoutResponse,
            451 => Status::UnavailableForLegalReasons,
            499 => Status::ClientClosedRequest,
            500 => Status::InternalServerError,
            501 => Status::NotImplemented,
            502 => Status::BadGateway,
            503 => Status::ServiceUnavailable,
            504 => Status::GatewayTimeout,
            505 => Status::HTTPVersionNotSupported,
            506 => Status::VariantAlsoNegotiates,
            507 => Status::InsufficientStorage,
            508 => Status::LoopDetected,
            510 => Status::NotExtended,
            511 => Status::NetworkAuthenticationRequired,
            599 => Status::NetworkConnectTimeoutError,
            _ => Status::Custom(format!("{} Custom", status_code)),
        }
    }
}

impl fmt::Display for Status {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let formatted = match self {
            Status::Continue => "100 Continue",
            Status::SwitchingProtocols => "101 Switching Protocols",
            Status::Processing => "102 Processing",
            Status::Ok => "200 OK",
            Status::Created => "201 Created",
            Status::Accepted => "202 Accepted",
            Status::NonAuthoritativeInformation => "203 Non-Authoritative Information",
            Status::NoContent => "204 No Content",
            Status::ResetContent => "205 Reset Content",
            Status::PartialContent => "206 Partial Content",
            Status::MultiStatus => "207 Multi-Status",
            Status::AlreadyReported => "208 Already Reported",
            Status::IMUsed => "226 IM Used",
            Status::MultipleChoices => "300 Multiple Choices",
            Status::MovedPermanently => "301 Moved Permanently",
            Status::Found => "302 Found",
            Status::SeeOther => "303 See Other",
            Status::NotModified => "304 Not Modified",
            Status::UseProxy => "305 Use Proxy",
            Status::TemporaryRedirect => "307 Temporary Redirect",
            Status::PermanentRedirect => "308 Permanent Redirect",
            Status::BadRequest => "400 Bad Request",
            Status::Unauthorized => "401 Unauthorized",
            Status::PaymentRequired => "402 Payment Required",
            Status::Forbidden => "403 Forbidden",
            Status::NotFound => "404 Not Found",
            Status::MethodNotAllowed => "405 Method Not Allowed",
            Status::NotAcceptable => "406 Not Acceptable",
            Status::ProxyAuthenticationRequired => "407 Proxy Authentication Required",
            Status::RequestTimeout => "408 Request Timeout",
            Status::Conflict => "409 Conflict",
            Status::Gone => "410 Gone",
            Status::LengthRequired => "411 Length Required",
            Status::PreconditionFailed => "412 Precondition Failed",
            Status::PayloadTooLarge => "413 Payload Too Large",
            Status::RequestURITooLong => "414 Request-URI Too Long",
            Status::UnsupportedMediaType => "415 Unsupported Media Type",
            Status::RequestedRangeNotSatisfiable => "416 Requested Range Not Satisfiable",
            Status::ExpectationFailed => "417 Expectation Failed",
            Status::ImATeapot => "418 I'm a teapot",
            Status::MisdirectedRequest => "421 Misdirected Request",
            Status::UnprocessableEntity => "422 Unprocessable Entity",
            Status::Locked => "423 Locked",
            Status::FailedDependency => "424 Failed Dependency",
            Status::UpgradeRequired => "426 Upgrade Required",
            Status::PreconditionRequired => "428 Precondition Required",
            Status::TooManyRequests => "429 Too Many Requests",
            Status::RequestHeaderFieldsTooLarge => "431 Request Header Fields Too Large",
            Status::ConnectionClosedWithoutResponse => "444 Connection Closed Without Response",
            Status::UnavailableForLegalReasons => "451 Unavailable For Legal Reasons",
            Status::ClientClosedRequest => "499 Client Closed Request",
            Status::InternalServerError => "500 Internal Server Error",
            Status::NotImplemented => "501 Not Implemented",
            Status::BadGateway => "502 Bad Gateway",
            Status::ServiceUnavailable => "503 Service Unavailable",
            Status::GatewayTimeout => "504 Gateway Timeout",
            Status::HTTPVersionNotSupported => "505 HTTP Version Not Supported",
            Status::VariantAlsoNegotiates => "506 Variant Also Negotiates",
            Status::InsufficientStorage => "507 Insufficient Storage",
            Status::LoopDetected => "508 Loop Detected",
            Status::NotExtended => "510 Not Extended",
            Status::NetworkAuthenticationRequired => "511 Network Authentication Required",
            Status::NetworkConnectTimeoutError => "599 Network Connect Timeout Error",
            Status::Custom(ref status_code) => status_code,
        };

        write!(f, "{}", formatted)
    }
}

//! HAR (HTTP Archive) 1.2 data structures.
//!
//! These types model the [HAR 1.2 specification](http://www.softwareishard.com/blog/har-12-spec/).
//! Fields are leniently parsed — many are `Option` even when the spec says
//! required, because real-world HAR files are often inconsistent.

use serde::{Deserialize, Serialize};

/// Root of a HAR file.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Har {
    /// The HAR log containing all entries.
    pub log: Log,
}

/// Top-level log object containing the HAR data.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Log {
    /// HAR format version (e.g. "1.2").
    pub version: String,
    /// Software that created the HAR file.
    pub creator: Creator,
    /// Optional browser info.
    #[serde(default)]
    pub browser: Option<Browser>,
    /// Optional list of pages (page groupings).
    #[serde(default)]
    pub pages: Option<Vec<Page>>,
    /// The HTTP request/response entries.
    pub entries: Vec<Entry>,
    /// Optional free-form comment.
    #[serde(default)]
    pub comment: Option<String>,
}

/// Software that generated the HAR file.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Creator {
    /// Application name (e.g. "Chrome").
    pub name: String,
    /// Application version.
    pub version: String,
    /// Optional comment.
    #[serde(default)]
    pub comment: Option<String>,
}

/// Browser info embedded in the HAR.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Browser {
    /// Browser name.
    pub name: String,
    /// Browser version.
    pub version: String,
    /// Optional comment.
    #[serde(default)]
    pub comment: Option<String>,
}

/// A page view grouping multiple entries.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Page {
    /// ISO 8601 start time.
    #[serde(rename = "startedDateTime")]
    pub started_date_time: String,
    /// Unique page identifier (referenced by entries).
    pub id: String,
    /// Page title.
    pub title: String,
    /// Additional page timing info.
    #[serde(rename = "pageTimings")]
    pub page_timings: PageTimings,
    /// Optional comment.
    #[serde(default)]
    pub comment: Option<String>,
}

/// Page-level timing information.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PageTimings {
    /// Optional content loaded time in ms.
    #[serde(rename = "onContentLoad", default)]
    pub on_content_load: Option<f64>,
    /// Optional page load time in ms.
    #[serde(rename = "onLoad", default)]
    pub on_load: Option<f64>,
    /// Optional comment.
    #[serde(default)]
    pub comment: Option<String>,
}

/// A single HTTP request-response pair.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Entry {
    /// ISO 8601 request start time.
    #[serde(rename = "startedDateTime")]
    pub started_date_time: String,
    /// Total elapsed time in milliseconds.
    pub time: f64,
    /// The HTTP request.
    pub request: Request,
    /// The HTTP response.
    pub response: Response,
    /// Cache state info.
    pub cache: Cache,
    /// Detailed timing breakdown in milliseconds.
    pub timings: Timings,
    /// Optional server IP address.
    #[serde(default, rename = "serverIPAddress")]
    pub server_ip_address: Option<String>,
    /// Optional connection identifier.
    #[serde(default)]
    pub connection: Option<String>,
    /// Optional page reference (matches a Page id).
    #[serde(default, rename = "pageref")]
    pub pageref: Option<String>,
    /// Optional comment.
    #[serde(default)]
    pub comment: Option<String>,
}

/// An HTTP request.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Request {
    /// HTTP method: GET, POST, PUT, DELETE, etc.
    pub method: String,
    /// Full request URL.
    pub url: String,
    /// HTTP version (e.g. "HTTP/1.1").
    #[serde(rename = "httpVersion")]
    pub http_version: String,
    /// Request cookies.
    #[serde(default)]
    pub cookies: Vec<Cookie>,
    /// Request headers.
    #[serde(default)]
    pub headers: Vec<Header>,
    /// Parsed query string parameters.
    #[serde(default, rename = "queryString")]
    pub query_string: Vec<QueryParam>,
    /// POST data (for POST/PUT requests).
    #[serde(default, rename = "postData")]
    pub post_data: Option<PostData>,
    /// Total header size in bytes.
    #[serde(rename = "headersSize")]
    pub headers_size: i64,
    /// Total body size in bytes.
    #[serde(rename = "bodySize")]
    pub body_size: i64,
    /// Optional comment.
    #[serde(default)]
    pub comment: Option<String>,
}

/// An HTTP response.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Response {
    /// HTTP status code.
    pub status: u16,
    /// HTTP status text (e.g. "OK", "Not Found").
    #[serde(rename = "statusText")]
    pub status_text: String,
    /// HTTP version.
    #[serde(rename = "httpVersion")]
    pub http_version: String,
    /// Response cookies.
    #[serde(default)]
    pub cookies: Vec<Cookie>,
    /// Response headers.
    #[serde(default)]
    pub headers: Vec<Header>,
    /// Response body metadata.
    pub content: Content,
    /// Redirect target URL (for 3xx responses).
    #[serde(default, rename = "redirectURL")]
    pub redirect_url: String,
    /// Total header size in bytes.
    #[serde(rename = "headersSize")]
    pub headers_size: i64,
    /// Total body size in bytes.
    #[serde(rename = "bodySize")]
    pub body_size: i64,
    /// Optional comment.
    #[serde(default)]
    pub comment: Option<String>,
}

/// Response content metadata.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Content {
    /// Content size in bytes.
    pub size: u64,
    /// Optional compressed size.
    #[serde(default)]
    pub compression: Option<u64>,
    /// MIME type (e.g. "application/json").
    #[serde(rename = "mimeType")]
    pub mime_type: String,
    /// Optional full response body text.
    #[serde(default)]
    pub text: Option<String>,
    /// Optional content encoding.
    #[serde(default)]
    pub encoding: Option<String>,
    /// Optional comment.
    #[serde(default)]
    pub comment: Option<String>,
}

/// Detailed timing breakdown for a request in milliseconds.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Timings {
    /// Time spent in queue / blocked.
    #[serde(default)]
    pub blocked: Option<f64>,
    /// DNS resolution time.
    #[serde(default)]
    pub dns: Option<f64>,
    /// TCP connect time.
    #[serde(default)]
    pub connect: Option<f64>,
    /// Request send time.
    pub send: f64,
    /// Time waiting for first response byte.
    pub wait: f64,
    /// Response receive time.
    pub receive: f64,
    /// SSL/TLS handshake time.
    #[serde(default)]
    pub ssl: Option<f64>,
    /// Optional comment.
    #[serde(default)]
    pub comment: Option<String>,
}

/// Cache state before and after the request.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Cache {
    /// Optional cache state before request.
    #[serde(default, rename = "beforeRequest")]
    pub before_request: Option<CacheState>,
    /// Optional cache state after request.
    #[serde(default, rename = "afterRequest")]
    pub after_request: Option<CacheState>,
    /// Optional comment.
    #[serde(default)]
    pub comment: Option<String>,
}

/// Cache entry state.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CacheState {
    /// Optional expiration timestamp.
    #[serde(default)]
    pub expires: Option<String>,
    /// Last access timestamp.
    #[serde(rename = "lastAccess")]
    pub last_access: String,
    /// `ETag` value.
    #[serde(rename = "eTag")]
    pub e_tag: String,
    /// Hit count.
    #[serde(rename = "hitCount")]
    pub hit_count: u64,
    /// Optional comment.
    #[serde(default)]
    pub comment: Option<String>,
}

/// An HTTP cookie.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Cookie {
    /// Cookie name.
    pub name: String,
    /// Cookie value.
    pub value: String,
    /// Optional path.
    #[serde(default)]
    pub path: Option<String>,
    /// Optional domain.
    #[serde(default)]
    pub domain: Option<String>,
    /// Optional expiration.
    #[serde(default)]
    pub expires: Option<String>,
    /// Optional `HttpOnly` flag.
    #[serde(default, rename = "httpOnly")]
    pub http_only: Option<bool>,
    /// Optional Secure flag.
    #[serde(default)]
    pub secure: Option<bool>,
    /// Optional comment.
    #[serde(default)]
    pub comment: Option<String>,
}

/// An HTTP header.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Header {
    /// Header name.
    pub name: String,
    /// Header value.
    pub value: String,
    /// Optional comment.
    #[serde(default)]
    pub comment: Option<String>,
}

/// A parsed query string parameter.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct QueryParam {
    /// Parameter name.
    pub name: String,
    /// Parameter value.
    pub value: String,
    /// Optional comment.
    #[serde(default)]
    pub comment: Option<String>,
}

/// POST data (body) of a request.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PostData {
    /// Content MIME type.
    #[serde(rename = "mimeType")]
    pub mime_type: String,
    /// Body text.
    pub text: String,
    /// Parsed form parameters (for form submissions).
    #[serde(default)]
    pub params: Option<Vec<PostParam>>,
    /// Optional comment.
    #[serde(default)]
    pub comment: Option<String>,
}

/// A POST form parameter.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PostParam {
    /// Parameter name.
    pub name: String,
    /// Optional parameter value.
    #[serde(default)]
    pub value: Option<String>,
    /// Optional uploaded file name.
    #[serde(default, rename = "fileName")]
    pub file_name: Option<String>,
    /// Optional content type for file uploads.
    #[serde(default, rename = "contentType")]
    pub content_type: Option<String>,
    /// Optional comment.
    #[serde(default)]
    pub comment: Option<String>,
}

//! Live-capture types.

use std::collections::HashMap;

use chromiumoxide::cdp::browser_protocol::network::RequestId;
use serde::Serialize;

pub struct StagedRequest {
    pub method: String,
    pub request_headers: Vec<(String, String)>,
}

pub struct StagedResponse {
    pub request_id: RequestId,
    pub url: String,
    pub status: u16,
    pub status_text: String,
    pub response_headers: Vec<(String, String)>,
    pub mime_type: String,
    pub encoded_data_length: f64,
}

#[derive(Debug, Serialize)]
pub struct CaptureEntry {
    pub url: String,
    pub method: String,
    pub status: u16,
    pub status_text: String,
    pub mime_type: String,
    pub size: f64,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub body: Option<String>,
    pub request_headers: HashMap<String, String>,
    pub response_headers: HashMap<String, String>,
}

//! Live-capture types.

use std::collections::HashMap;

use chromiumoxide::cdp::browser_protocol::network::RequestId;
use serde::Serialize;

pub(crate) struct StagedRequest {
    pub(crate) method: String,
    pub(crate) request_headers: Vec<(String, String)>,
}

pub(crate) struct StagedResponse {
    pub(crate) request_id: RequestId,
    pub(crate) url: String,
    pub(crate) status: u16,
    pub(crate) status_text: String,
    pub(crate) response_headers: Vec<(String, String)>,
    pub(crate) mime_type: String,
    pub(crate) encoded_data_length: f64,
}

#[derive(Debug, Serialize)]
pub(crate) struct CaptureEntry {
    pub(crate) url: String,
    pub(crate) method: String,
    pub(crate) status: u16,
    pub(crate) status_text: String,
    pub(crate) mime_type: String,
    pub(crate) size: f64,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub(crate) body: Option<String>,
    pub(crate) request_headers: HashMap<String, String>,
    pub(crate) response_headers: HashMap<String, String>,
}

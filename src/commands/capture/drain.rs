//! Event stream draining functions.

use std::collections::HashMap;
use std::sync::Arc;
use std::time::Duration;

use chromiumoxide::cdp::browser_protocol::network::{
    EventLoadingFinished, EventRequestWillBeSent, EventResponseReceived,
    RequestId,
};
use futures::StreamExt;
use regex::Regex;
use serde::Serialize;
use tokio::time::timeout;

use super::types::{StagedRequest, StagedResponse};

/// Drain buffered request events into a staged map.
pub(super) async fn drain_requests(
    stream: &mut (impl StreamExt<Item = Arc<EventRequestWillBeSent>> + Unpin),
    url_filter: Option<&Regex>,
) -> HashMap<RequestId, StagedRequest> {
    let mut requests = HashMap::new();
    while let Ok(Some(event)) =
        timeout(Duration::from_millis(500), stream.next()).await
    {
        if let Some(filter) = url_filter {
            if !filter.is_match(&event.request.url) {
                continue;
            }
        }
        requests.insert(
            event.request_id.clone(),
            StagedRequest {
                method: event.request.method.clone(),
                request_headers: json_headers_to_vec(&event.request.headers),
            },
        );
    }
    requests
}

/// Drain buffered response events into a staged vec.
pub(super) async fn drain_responses(
    stream: &mut (impl StreamExt<Item = Arc<EventResponseReceived>> + Unpin),
    url_filter: Option<&Regex>,
) -> Vec<StagedResponse> {
    let mut responses = Vec::new();
    while let Ok(Some(event)) =
        timeout(Duration::from_millis(500), stream.next()).await
    {
        if let Some(filter) = url_filter {
            if !filter.is_match(&event.response.url) {
                continue;
            }
        }
        responses.push(StagedResponse {
            request_id: event.request_id.clone(),
            url: event.response.url.clone(),
            status: event.response.status as u16,
            status_text: event.response.status_text.clone(),
            response_headers: json_headers_to_vec(&event.response.headers),
            mime_type: event.response.mime_type.clone(),
            encoded_data_length: event.response.encoded_data_length,
        });
    }
    responses
}

/// Drain buffered loading-finished events into a staged vec.
pub(super) async fn drain_finished(
    stream: &mut (impl StreamExt<Item = Arc<EventLoadingFinished>> + Unpin),
) -> Vec<RequestId> {
    let mut finished = Vec::new();
    while let Ok(Some(event)) =
        timeout(Duration::from_millis(500), stream.next()).await
    {
        finished.push(event.request_id.clone());
    }
    finished
}

pub(super) fn json_headers_to_vec<T: Serialize>(
    headers: &T,
) -> Vec<(String, String)> {
    let json = serde_json::to_value(headers).unwrap_or_default();
    let mut pairs = Vec::new();
    if let Some(obj) = json.as_object() {
        for (k, v) in obj {
            let val = match v {
                serde_json::Value::String(s) => s.clone(),
                other => other.to_string(),
            };
            pairs.push((k.clone(), val));
        }
    }
    pairs
}

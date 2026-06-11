//! `harvey capture` — live HAR capture from Chrome via CDP.

use std::collections::HashMap;
use std::path::PathBuf;
use std::sync::Arc;
use std::time::Duration;

use anyhow::{Context, Result};
use chromiumoxide::browser::{Browser, BrowserConfig};
use chromiumoxide::cdp::browser_protocol::network::{
    EventLoadingFinished, EventRequestWillBeSent, EventResponseReceived,
    GetResponseBodyParams, RequestId,
};
use chromiumoxide::Page as ChromiumPage;
use clap::Args;
use futures::{future, FutureExt, StreamExt};
use regex::Regex;
use serde::Serialize;
use tabled::{
    builder::Builder,
    settings::{object::Rows, Color, Modify, Style},
};
use tokio::time::timeout;

use crate::cli::GlobalArgs;
use crate::output::OutputMode;

/// Live-capture network traffic from a URL via Chrome DevTools Protocol.
#[derive(Debug, Args)]
#[command(group = clap::ArgGroup::new("mode").args(["connect", "url"]).required(true))]
pub struct CaptureArgs {
    /// Target URL to capture network traffic from.
    /// Not needed when using --connect.
    #[arg(
        long,
        value_name = "URL",
        value_hint = clap::ValueHint::Url,
        value_parser = crate::validators::valid_url,
        help_heading = "Capture behavior"
    )]
    pub url: Option<String>,

    /// Maximum capture duration in seconds (default: 30).
    #[arg(
        long,
        value_name = "SECS",
        default_value_t = 30,
        help_heading = "Capture behavior"
    )]
    pub timeout: u64,

    /// Show the browser window instead of running headless.
    /// Ignored with --connect.
    #[arg(
        long,
        action = clap::ArgAction::SetTrue,
        conflicts_with = "connect",
        help_heading = "Browser config"
    )]
    pub no_headless: bool,

    /// Only capture requests whose URL matches this regex.
    #[arg(
        long,
        value_name = "REGEX",
        allow_hyphen_values = true,
        help_heading = "Capture behavior"
    )]
    pub filter_url: Option<String>,

    /// Fetch and include response body text in output.
    #[arg(long, action = clap::ArgAction::SetTrue, help_heading = "Capture behavior")]
    pub include_body: bool,

    /// Path to Chrome/Chromium/Helium executable (auto-detected if not set).
    /// Ignored with --connect.
    #[arg(
        long,
        value_name = "PATH",
        env = "CHROME_PATH",
        value_hint = clap::ValueHint::FilePath,
        value_parser = crate::validators::existing_file,
        conflicts_with = "connect",
        help_heading = "Browser config"
    )]
    pub chrome: Option<PathBuf>,

    /// Path to a Chrome user data directory for persistent profile
    /// (extensions, cookies, logins). Chrome must not already be
    /// running with this profile. Conflicts with --profile and --connect.
    #[arg(
        long,
        value_name = "PATH",
        env = "CHROME_USER_DATA",
        value_hint = clap::ValueHint::DirPath,
        value_parser = crate::validators::existing_dir,
        conflicts_with_all = ["profile", "connect"],
        help_heading = "Browser config"
    )]
    pub user_data_dir: Option<PathBuf>,

    /// Auto-discover and use your default Chrome/Chromium/Helium profile
    /// directory. Optionally specify a profile name (e.g. "Default",
    /// "Profile 1"). Conflicts with --user-data-dir and --connect.
    #[arg(
        long,
        value_name = "NAME",
        num_args = 0..=1,
        default_missing_value = "Default",
        conflicts_with_all = ["user_data_dir", "connect"],
        help_heading = "Browser config"
    )]
    pub profile: Option<String>,

    /// Keep capturing until Ctrl+C instead of stopping after initial load.
    #[arg(long, short = 'w', action = clap::ArgAction::SetTrue, help_heading = "Capture behavior")]
    pub watch: bool,

    /// Connect to an already-running browser via remote debugging URL.
    /// Defaults to <http://127.0.0.1:9222>. The target browser must be
    /// started with --remote-debugging-port. Conflicts with all launch args.
    #[arg(
        long,
        value_name = "URL",
        num_args = 0..=1,
        default_missing_value = "http://127.0.0.1:9222",
        value_hint = clap::ValueHint::Url,
        value_parser = crate::validators::valid_url,
        conflicts_with_all = [
            "debugging_port",
            "url",
            "no_headless",
            "chrome",
            "user_data_dir",
            "profile"
        ],
        help_heading = "Remote connect"
    )]
    pub connect: Option<String>,

    /// Start the browser with a specific remote debugging port
    /// (e.g. 9222). Conflicts with --connect.
    #[arg(
        long,
        value_name = "PORT",
        conflicts_with = "connect",
        help_heading = "Browser config"
    )]
    pub debugging_port: Option<u16>,
}

struct StagedRequest {
    method: String,
    request_headers: Vec<(String, String)>,
}

struct StagedResponse {
    request_id: RequestId,
    url: String,
    status: u16,
    status_text: String,
    response_headers: Vec<(String, String)>,
    mime_type: String,
    encoded_data_length: f64,
}

#[derive(Debug, Serialize)]
struct CaptureEntry {
    url: String,
    method: String,
    status: u16,
    status_text: String,
    mime_type: String,
    size: f64,
    #[serde(skip_serializing_if = "Option::is_none")]
    body: Option<String>,
    request_headers: HashMap<String, String>,
    response_headers: HashMap<String, String>,
}

/// Run the capture command.
///
/// # Errors
///
/// Returns an error if the browser cannot be launched, the page
/// cannot be navigated, or output cannot be written.
pub async fn run(args: &CaptureArgs, global: &GlobalArgs) -> Result<()> {
    let mode = OutputMode::from_args(global.json);
    let url_filter = if let Some(ref pattern) = args.filter_url {
        Some(Regex::new(pattern).context("invalid --filter-url regex")?)
    } else {
        None
    };

    if let Some(ref ws_url) = args.connect {
        return run_connect(args, global, mode, ws_url, url_filter).await;
    }

    // Launch mode.
    let mut config =
        BrowserConfig::builder().no_sandbox().window_size(1280, 900);
    if args.no_headless {
        config = config.with_head();
    }
    if let Some(ref chrome_path) = args.chrome {
        config = config.chrome_executable(chrome_path);
    }
    if let Some(ref data_dir) = args.user_data_dir {
        config = config.user_data_dir(data_dir);
    }
    if let Some(ref profile_name) = args.profile {
        if let Some(data_dir) = discover_chrome_user_data() {
            tracing::info!(
                "using Chrome profile: {} (profile: {profile_name})",
                data_dir.display()
            );
            config = config.user_data_dir(&data_dir);
        } else {
            tracing::warn!(
                "could not auto-discover Chrome user data directory"
            );
        }
    }
    if let Some(port) = args.debugging_port {
        config = config.arg(format!("--remote-debugging-port={port}"));
    }
    let (browser, mut handler) = Browser::launch(
        config
            .build()
            .map_err(|e| anyhow::anyhow!("browser config error: {e}"))?,
    )
    .await?;
    tokio::spawn(async move { while handler.next().await.is_some() {} });

    let page = browser.new_page("about:blank").await?;
    let mut req_stream =
        page.event_listener::<EventRequestWillBeSent>().await?;
    let mut resp_stream =
        page.event_listener::<EventResponseReceived>().await?;
    let mut finished_stream =
        page.event_listener::<EventLoadingFinished>().await?;
    let page = Arc::new(page);

    let url = args
        .url
        .as_deref()
        .context("--url is required in launch mode")?;
    let nav = timeout(Duration::from_secs(args.timeout), page.goto(url)).await;
    nav.map_err(|_| {
        anyhow::anyhow!("navigation timed out after {}s", args.timeout)
    })?
    .context("page navigation failed")?;
    tokio::time::sleep(Duration::from_secs(2)).await;

    let (requests, responses, finished) = if args.watch {
        run_watch_loop(
            &mut req_stream,
            &mut resp_stream,
            &mut finished_stream,
            url_filter.as_ref(),
        )
        .await
    } else {
        run_once(
            &mut req_stream,
            &mut resp_stream,
            &mut finished_stream,
            url_filter.as_ref(),
        )
        .await
    };

    output_entries(
        mode,
        &requests,
        &responses,
        &finished,
        &page,
        args.include_body,
    )
    .await
}

async fn run_connect(
    args: &CaptureArgs,
    _global: &GlobalArgs,
    mode: OutputMode,
    ws_url: &str,
    url_filter: Option<Regex>,
) -> Result<()> {
    let (browser, mut handler) = Browser::connect(ws_url).await.context(
        "failed to connect to browser.\n\
         Make sure it was started with --remote-debugging-port, e.g.:\n  \
         /Applications/Helium.app/Contents/MacOS/Helium --remote-debugging-port=9222",
    )?;
    tokio::spawn(async move { while handler.next().await.is_some() {} });

    tracing::info!("connected to browser at {ws_url}");

    let page = if let Some(url) = &args.url {
        let page = browser.new_page("about:blank").await?;
        let nav =
            timeout(Duration::from_secs(args.timeout), page.goto(url)).await;
        nav.map_err(|_| {
            anyhow::anyhow!("navigation timed out after {}s", args.timeout)
        })?
        .context("page navigation failed")?;
        page
    } else {
        let pages = browser.pages().await?;
        pages
            .into_iter()
            .next()
            .context("no open pages in browser")?
    };
    let page = Arc::new(page);

    let mut req_stream =
        page.event_listener::<EventRequestWillBeSent>().await?;
    let mut resp_stream =
        page.event_listener::<EventResponseReceived>().await?;
    let mut finished_stream =
        page.event_listener::<EventLoadingFinished>().await?;

    tracing::info!("watching — press Ctrl+C to stop");
    let (requests, responses, finished) = run_watch_loop(
        &mut req_stream,
        &mut resp_stream,
        &mut finished_stream,
        url_filter.as_ref(),
    )
    .await;

    output_entries(
        mode,
        &requests,
        &responses,
        &finished,
        &page,
        args.include_body,
    )
    .await
}

/// Build entries from captured data and render.
async fn output_entries(
    mode: OutputMode,
    requests: &HashMap<RequestId, StagedRequest>,
    responses: &[StagedResponse],
    finished: &[RequestId],
    page: &Arc<ChromiumPage>,
    include_body: bool,
) -> Result<()> {
    if matches!(mode, OutputMode::Human) {
        tracing::info!(
            "captured {} requests, {} responses",
            requests.len(),
            responses.len()
        );
    }

    let entries =
        build_entries(requests, responses, finished, page, include_body).await;

    if entries.is_empty() {
        tracing::warn!("no matching requests captured");
        anyhow::bail!("NO_RESULTS");
    }

    match mode {
        OutputMode::Human => render_human(&entries),
        OutputMode::Json => render_jsonl(&entries),
    }
}

/// One-shot capture: drain buffered events once.
async fn run_once(
    req_stream: &mut (impl StreamExt<Item = Arc<EventRequestWillBeSent>> + Unpin),
    resp_stream: &mut (impl StreamExt<Item = Arc<EventResponseReceived>> + Unpin),
    finished_stream: &mut (impl StreamExt<Item = Arc<EventLoadingFinished>> + Unpin),
    url_filter: Option<&Regex>,
) -> (
    HashMap<RequestId, StagedRequest>,
    Vec<StagedResponse>,
    Vec<RequestId>,
) {
    tokio::join!(
        drain_requests(req_stream, url_filter),
        drain_responses(resp_stream, url_filter),
        drain_finished(finished_stream),
    )
}

/// Watch loop: keep draining until Ctrl+C.
async fn run_watch_loop(
    req_stream: &mut (impl StreamExt<Item = Arc<EventRequestWillBeSent>> + Unpin),
    resp_stream: &mut (impl StreamExt<Item = Arc<EventResponseReceived>> + Unpin),
    finished_stream: &mut (impl StreamExt<Item = Arc<EventLoadingFinished>> + Unpin),
    url_filter: Option<&Regex>,
) -> (
    HashMap<RequestId, StagedRequest>,
    Vec<StagedResponse>,
    Vec<RequestId>,
) {
    let mut requests = HashMap::new();
    let mut responses = Vec::new();
    let mut finished = Vec::new();

    tracing::info!("watching — press Ctrl+C to stop");

    loop {
        if tokio::signal::ctrl_c().now_or_never().is_some() {
            tracing::info!("interrupted, stopping capture");
            break;
        }

        let (new_requests, new_responses, new_finished) = tokio::join!(
            drain_requests(req_stream, url_filter),
            drain_responses(resp_stream, url_filter),
            drain_finished(finished_stream),
        );

        requests.extend(new_requests);
        responses.extend(new_responses);
        finished.extend(new_finished);

        tokio::time::sleep(Duration::from_millis(200)).await;
    }

    (requests, responses, finished)
}

/// Build output entries by pairing staged requests with responses.
async fn build_entries(
    requests: &HashMap<RequestId, StagedRequest>,
    responses: &[StagedResponse],
    finished: &[RequestId],
    page: &Arc<ChromiumPage>,
    include_body: bool,
) -> Vec<CaptureEntry> {
    let body_futures: Vec<_> = responses
        .iter()
        .map(|resp| {
            let page = Arc::clone(page);
            let request_id = resp.request_id.clone();
            let need_body = include_body && finished.contains(&resp.request_id);
            async move {
                if need_body {
                    fetch_body_inner(&page, &request_id).await
                } else {
                    None
                }
            }
        })
        .collect();

    let bodies = future::join_all(body_futures).await;

    let mut entries = Vec::with_capacity(responses.len());
    for (i, resp) in responses.iter().enumerate() {
        let req = requests.get(&resp.request_id);
        let req_headers: HashMap<String, String> = req
            .map(|r| r.request_headers.iter().cloned().collect())
            .unwrap_or_default();

        entries.push(CaptureEntry {
            url: resp.url.clone(),
            method: req.map_or_else(|| "UNKNOWN".into(), |r| r.method.clone()),
            status: resp.status,
            status_text: resp.status_text.clone(),
            mime_type: resp.mime_type.clone(),
            size: resp.encoded_data_length,
            body: bodies[i].clone(),
            request_headers: req_headers,
            response_headers: resp.response_headers.iter().cloned().collect(),
        });
    }
    entries
}

async fn fetch_body_inner(
    page: &Arc<ChromiumPage>,
    request_id: &RequestId,
) -> Option<String> {
    let params = GetResponseBodyParams::new(request_id.clone());
    match page.activate().await.ok()?.execute(params).await {
        Ok(result) => {
            if result.base64_encoded {
                Some(format!("[base64: {} bytes]", result.body.len()))
            } else {
                Some(result.body.clone())
            }
        }
        Err(_) => None,
    }
}

/// Drain buffered request events into a staged map.
async fn drain_requests(
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
async fn drain_responses(
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
async fn drain_finished(
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

fn json_headers_to_vec<T: Serialize>(headers: &T) -> Vec<(String, String)> {
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

fn render_human(entries: &[CaptureEntry]) -> Result<()> {
    let mut builder = Builder::default();
    builder.push_record(["Method", "Status", "URL", "Size", "Content-Type"]);

    for e in entries {
        builder.push_record([
            e.method.clone(),
            e.status.to_string(),
            truncate_url(&e.url, 80),
            format_size(e.size),
            e.mime_type.clone(),
        ]);
    }

    let mut table = builder.build();
    table
        .with(Style::rounded())
        .with(Modify::new(Rows::first()).with(Color::BOLD));

    let mut stdout = std::io::stdout();
    std::io::Write::write_all(&mut stdout, table.to_string().as_bytes())
        .context("failed to write table to stdout")?;
    std::io::Write::write_all(&mut stdout, b"\n")
        .context("failed to write newline")?;
    Ok(())
}

fn render_jsonl(entries: &[CaptureEntry]) -> Result<()> {
    let mut stdout = std::io::BufWriter::new(std::io::stdout());
    for entry in entries {
        let line = serde_json::to_string(entry)
            .context("failed to serialize capture entry")?;
        std::io::Write::write_all(&mut stdout, line.as_bytes())
            .context("failed to write entry to stdout")?;
        std::io::Write::write_all(&mut stdout, b"\n")
            .context("failed to write newline")?;
    }
    Ok(())
}

fn truncate_url(url: &str, max_len: usize) -> String {
    if url.len() <= max_len {
        url.to_owned()
    } else {
        format!("{}…", &url[..url.floor_char_boundary(max_len - 1)])
    }
}

/// Auto-discover a Chrome/Chromium/Helium user data directory.
///
/// Checks known locations on macOS and Linux in order of preference.
fn discover_chrome_user_data() -> Option<std::path::PathBuf> {
    let home = std::env::var("HOME").ok()?;
    let home = std::path::PathBuf::from(home);

    let candidates: &[&str] = {
        #[cfg(target_os = "macos")]
        {
            &[
                "Library/Application Support/Google/Chrome",
                "Library/Application Support/Chromium",
                "Library/Application Support/net.imput.helium", // Helium
            ]
        }
        #[cfg(target_os = "linux")]
        {
            &[
                ".config/google-chrome",
                ".config/chromium",
                ".config/helium",
            ]
        }
        #[cfg(not(any(target_os = "macos", target_os = "linux")))]
        {
            &[]
        }
    };

    for path in candidates {
        let full = home.join(path);
        if full.exists() {
            return Some(full);
        }
    }
    None
}

fn format_size(bytes: f64) -> String {
    const KB: f64 = 1024.0;
    const MB: f64 = KB * 1024.0;
    if bytes >= MB {
        format!("{:.1} MB", bytes / MB)
    } else if bytes >= KB {
        format!("{:.1} KB", bytes / KB)
    } else {
        format!("{bytes:.0} B")
    }
}

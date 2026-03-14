//! Chromecast / Google Cast V2 implementation.
//!
//! Uses only pure-Rust or OS-native dependencies:
//!   - `mdns` crate for mDNS device discovery
//!   - `tokio-native-tls` (Windows Schannel) for TLS  (no OpenSSL, no Perl)
//!   - `prost` derive macros for Cast V2 protobuf     (no protoc)

use std::net::IpAddr;
use std::time::Duration;

use futures_util::StreamExt;
use prost::Message as ProstMsg;
use serde_json::{json, Value};
use tauri::State;
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::TcpStream;
use tokio_native_tls::native_tls;

use crate::{AppState, CastSessionInfo};

// ── Cast V2 minimal protobuf message ─────────────────────────────────────────

#[derive(Clone, PartialEq, prost::Message)]
struct CastMsg {
    #[prost(int32, required, tag = "1")]
    protocol_version: i32,
    #[prost(string, required, tag = "2")]
    source_id: String,
    #[prost(string, required, tag = "3")]
    destination_id: String,
    #[prost(string, required, tag = "4")]
    namespace: String,
    #[prost(int32, required, tag = "5")]
    payload_type: i32,
    #[prost(string, optional, tag = "6")]
    payload_utf8: Option<String>,
}

type TlsStream = tokio_native_tls::TlsStream<TcpStream>;

const NS_CONN: &str = "urn:x-cast:com.google.cast.tp.connection";
const NS_RECV: &str = "urn:x-cast:com.google.cast.receiver";
const NS_MEDIA: &str = "urn:x-cast:com.google.cast.media";
const MEDIA_RECEIVER_APP: &str = "CC1AD845";

// ── Device info ───────────────────────────────────────────────────────────────

#[derive(serde::Serialize, serde::Deserialize, Clone, Debug)]
#[serde(rename_all = "camelCase")]
pub struct CastDeviceInfo {
    pub name: String,
    pub addr: String,
    pub port: u16,
}

// ── mDNS discovery ────────────────────────────────────────────────────────────

#[tauri::command]
pub async fn cast_discover() -> Result<Vec<CastDeviceInfo>, String> {
    use std::collections::HashMap;

    let stream = mdns::discover::all("_googlecast._tcp.local", Duration::from_secs(3))
        .map_err(|e| e.to_string())?
        .listen();
    tokio::pin!(stream);

    let deadline = tokio::time::Instant::now() + Duration::from_secs(5);
    let mut devices: HashMap<String, CastDeviceInfo> = HashMap::new();

    loop {
        let rem = deadline.saturating_duration_since(tokio::time::Instant::now());
        if rem.is_zero() { break; }
        match tokio::time::timeout(rem, stream.next()).await {
            Ok(Some(Ok(response))) => {
                let ip = match response.ip_addr() {
                    Some(IpAddr::V4(ip)) => ip,
                    _ => continue,
                };
                let port = match response.port() {
                    Some(p) => p,
                    None => continue,
                };
                let name = response
                    .txt_records()
                    .find_map(|s| s.strip_prefix("fn="))
                    .map(|s| s.to_string())
                    .unwrap_or_else(|| ip.to_string());
                let key = format!("{}:{}", ip, port);
                devices.entry(key).or_insert(CastDeviceInfo { name, addr: ip.to_string(), port });
            }
            _ => break,
        }
    }

    Ok(devices.into_values().collect())
}

// ── TLS connection ────────────────────────────────────────────────────────────

async fn cast_connect(addr: &str, port: u16) -> Result<TlsStream, String> {
    let cx = native_tls::TlsConnector::builder()
        .danger_accept_invalid_certs(true)
        .danger_accept_invalid_hostnames(true)
        .build()
        .map_err(|e| e.to_string())?;
    let cx = tokio_native_tls::TlsConnector::from(cx);
    let tcp = TcpStream::connect((addr, port)).await.map_err(|e| e.to_string())?;
    cx.connect("na", tcp).await.map_err(|e| e.to_string())
}

// ── Message I/O ───────────────────────────────────────────────────────────────

async fn send_msg(stream: &mut TlsStream, src: &str, dst: &str, ns: &str, payload: Value) -> Result<(), String> {
    let msg = CastMsg {
        source_id: src.to_string(),
        destination_id: dst.to_string(),
        namespace: ns.to_string(),
        payload_type: 0,
        payload_utf8: Some(payload.to_string()),
        ..Default::default()
    };
    let mut buf = Vec::new();
    msg.encode(&mut buf).map_err(|e| e.to_string())?;
    let len = (buf.len() as u32).to_be_bytes();
    stream.write_all(&len).await.map_err(|e| e.to_string())?;
    stream.write_all(&buf).await.map_err(|e| e.to_string())?;
    stream.flush().await.map_err(|e| e.to_string())?;
    Ok(())
}

async fn recv_msg(stream: &mut TlsStream) -> Result<(String, Value), String> {
    let mut len_buf = [0u8; 4];
    stream.read_exact(&mut len_buf).await.map_err(|e| e.to_string())?;
    let len = u32::from_be_bytes(len_buf) as usize;
    if len > 65_536 { return Err("Cast message too large".into()); }
    let mut buf = vec![0u8; len];
    stream.read_exact(&mut buf).await.map_err(|e| e.to_string())?;
    let msg = CastMsg::decode(buf.as_slice()).map_err(|e| e.to_string())?;
    let payload = msg.payload_utf8.as_deref().unwrap_or("{}");
    let val: Value = serde_json::from_str(payload).unwrap_or_default();
    Ok((msg.namespace, val))
}

async fn recv_until(
    stream: &mut TlsStream,
    ns_filter: &str,
    predicate: impl Fn(&Value) -> bool,
    timeout_secs: u64,
) -> Result<Value, String> {
    let deadline = tokio::time::Instant::now() + Duration::from_secs(timeout_secs);
    loop {
        let rem = deadline.saturating_duration_since(tokio::time::Instant::now());
        if rem.is_zero() { return Err("Timeout waiting for Cast response".into()); }
        let (ns, val) = tokio::time::timeout(rem, recv_msg(stream))
            .await
            .map_err(|_| "Timeout".to_string())??;
        if ns == "urn:x-cast:com.google.cast.tp.heartbeat" {
            let _ = send_msg(stream, "sender-0", "receiver-0", &ns, json!({"type":"PONG"})).await;
            continue;
        }
        if ns == ns_filter && predicate(&val) {
            return Ok(val);
        }
    }
}

// ── cast_play ─────────────────────────────────────────────────────────────────

#[tauri::command]
pub async fn cast_play(
    state: State<'_, AppState>,
    device_name: String,
    device_addr: String,
    device_port: u16,
    stream_url: String,
    title: String,
    artist: String,
    cover_url: String,
) -> Result<(), String> {
    let mut tls = cast_connect(&device_addr, device_port).await?;

    send_msg(&mut tls, "sender-0", "receiver-0", NS_CONN, json!({"type":"CONNECT"})).await?;
    send_msg(&mut tls, "sender-0", "receiver-0", NS_RECV,
        json!({"type":"LAUNCH","requestId":1,"appId":MEDIA_RECEIVER_APP})).await?;

    let recv_status = recv_until(&mut tls, NS_RECV, |v| {
        v["type"].as_str() == Some("RECEIVER_STATUS")
            && v["status"]["applications"].as_array()
                .map(|apps| apps.iter().any(|a| a["appId"].as_str() == Some(MEDIA_RECEIVER_APP)))
                .unwrap_or(false)
    }, 20).await?;

    let app = recv_status["status"]["applications"]
        .as_array()
        .and_then(|apps| apps.iter().find(|a| a["appId"].as_str() == Some(MEDIA_RECEIVER_APP)))
        .ok_or("App not found in RECEIVER_STATUS")?;

    let transport_id = app["transportId"].as_str().ok_or("Missing transportId")?.to_string();
    let session_id = app["sessionId"].as_str().unwrap_or("").to_string();

    send_msg(&mut tls, "sender-0", &transport_id, NS_CONN, json!({"type":"CONNECT"})).await?;

    let images = if cover_url.is_empty() { json!([]) } else { json!([{"url": cover_url}]) };
    send_msg(&mut tls, "sender-0", &transport_id, NS_MEDIA, json!({
        "type": "LOAD",
        "requestId": 2,
        "media": {
            "contentId": stream_url,
            "contentType": "audio/mpeg",
            "streamType": "BUFFERED",
            "metadata": { "metadataType": 3, "title": title, "artist": artist, "images": images }
        },
        "autoplay": true,
        "currentTime": 0
    })).await?;

    let media_status = recv_until(&mut tls, NS_MEDIA,
        |v| v["type"].as_str() == Some("MEDIA_STATUS"), 15).await?;

    let media_session_id = media_status["status"]
        .as_array().and_then(|a| a.first())
        .and_then(|s| s["mediaSessionId"].as_i64())
        .unwrap_or(1) as i32;

    *state.cast_session.lock().map_err(|e| e.to_string())? = Some(CastSessionInfo {
        device_name, device_addr, device_port, transport_id, session_id, media_session_id,
    });
    Ok(())
}

// ── Helper: reconnect + send a media command ──────────────────────────────────

async fn media_command(session: &CastSessionInfo, payload: Value) -> Result<(), String> {
    let mut tls = cast_connect(&session.device_addr, session.device_port).await?;
    send_msg(&mut tls, "sender-0", "receiver-0", NS_CONN, json!({"type":"CONNECT"})).await?;
    send_msg(&mut tls, "sender-0", &session.transport_id, NS_CONN, json!({"type":"CONNECT"})).await?;
    send_msg(&mut tls, "sender-0", &session.transport_id, NS_MEDIA, payload).await?;
    Ok(())
}

// ── cast_pause ────────────────────────────────────────────────────────────────

#[tauri::command]
pub async fn cast_pause(state: State<'_, AppState>) -> Result<(), String> {
    let session = state.cast_session.lock().map_err(|e| e.to_string())?.clone()
        .ok_or("No active cast session")?;
    media_command(&session, json!({"type":"PAUSE","requestId":3,"mediaSessionId":session.media_session_id})).await
}

// ── cast_resume ───────────────────────────────────────────────────────────────

#[tauri::command]
pub async fn cast_resume(state: State<'_, AppState>) -> Result<(), String> {
    let session = state.cast_session.lock().map_err(|e| e.to_string())?.clone()
        .ok_or("No active cast session")?;
    media_command(&session, json!({"type":"PLAY","requestId":4,"mediaSessionId":session.media_session_id})).await
}

// ── cast_stop ─────────────────────────────────────────────────────────────────

#[tauri::command]
pub async fn cast_stop(state: State<'_, AppState>) -> Result<(), String> {
    let session = { let mut l = state.cast_session.lock().map_err(|e| e.to_string())?; l.take() };
    if let Some(session) = session {
        let mut tls = cast_connect(&session.device_addr, session.device_port).await?;
        send_msg(&mut tls, "sender-0", "receiver-0", NS_CONN, json!({"type":"CONNECT"})).await?;
        send_msg(&mut tls, "sender-0", "receiver-0", NS_RECV,
            json!({"type":"STOP","requestId":5,"sessionId":session.session_id})).await?;
    }
    Ok(())
}

// ── cast_get_session ──────────────────────────────────────────────────────────

#[tauri::command]
pub async fn cast_get_session(state: State<'_, AppState>) -> Result<Option<CastSessionInfo>, String> {
    Ok(state.cast_session.lock().map_err(|e| e.to_string())?.clone())
}

// ── cast_get_status ───────────────────────────────────────────────────────────
// Returns (currentTime, playerState) — used by the UI to advance the seek bar.

#[derive(serde::Serialize, serde::Deserialize, Clone, Debug)]
#[serde(rename_all = "camelCase")]
pub struct CastPlaybackStatus {
    pub current_time: f64,
    pub player_state: String, // "PLAYING" | "PAUSED" | "IDLE" | "BUFFERING"
}

#[tauri::command]
pub async fn cast_get_status(state: State<'_, AppState>) -> Result<CastPlaybackStatus, String> {
    let session = state.cast_session.lock().map_err(|e| e.to_string())?.clone()
        .ok_or("No active cast session")?;
    let mut tls = cast_connect(&session.device_addr, session.device_port).await?;
    send_msg(&mut tls, "sender-0", "receiver-0", NS_CONN, json!({"type":"CONNECT"})).await?;
    send_msg(&mut tls, "sender-0", &session.transport_id, NS_CONN, json!({"type":"CONNECT"})).await?;
    send_msg(&mut tls, "sender-0", &session.transport_id, NS_MEDIA,
        json!({"type":"GET_STATUS","requestId":8,"mediaSessionId":session.media_session_id})).await?;

    let resp = recv_until(&mut tls, NS_MEDIA,
        |v| v["type"].as_str() == Some("MEDIA_STATUS"), 8).await?;

    let status = resp["status"].as_array().and_then(|a| a.first()).cloned()
        .unwrap_or_default();

    Ok(CastPlaybackStatus {
        current_time: status["currentTime"].as_f64().unwrap_or(0.0),
        player_state: status["playerState"].as_str().unwrap_or("IDLE").to_string(),
    })
}

// ── cast_set_volume ───────────────────────────────────────────────────────────
// level: 0.0 – 1.0

#[tauri::command]
pub async fn cast_set_volume(state: State<'_, AppState>, level: f64) -> Result<(), String> {
    let session = state.cast_session.lock().map_err(|e| e.to_string())?.clone()
        .ok_or("No active cast session")?;
    let mut tls = cast_connect(&session.device_addr, session.device_port).await?;
    send_msg(&mut tls, "sender-0", "receiver-0", NS_CONN, json!({"type":"CONNECT"})).await?;
    send_msg(&mut tls, "sender-0", "receiver-0", NS_RECV,
        json!({"type":"SET_VOLUME","requestId":6,"volume":{"level": level}})).await?;
    Ok(())
}

// ── cast_seek ─────────────────────────────────────────────────────────────────
// position: seconds (f64)

#[tauri::command]
pub async fn cast_seek(state: State<'_, AppState>, position: f64) -> Result<(), String> {
    let session = state.cast_session.lock().map_err(|e| e.to_string())?.clone()
        .ok_or("No active cast session")?;
    media_command(&session,
        json!({"type":"SEEK","requestId":7,"mediaSessionId":session.media_session_id,"currentTime":position})).await
}

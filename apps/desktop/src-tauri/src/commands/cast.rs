//! Chromecast / Google Cast V2 implementation backed by the `rust_cast` crate.
//!
//! Architecture
//! ────────────
//! A dedicated `std::thread` (the "cast actor") owns the `CastDevice` for the
//! lifetime of a cast session.  Tauri commands communicate with it via an
//! `mpsc::Sender<CastActorCmd>` stored in `AppState`.  Each command carries a
//! `tokio::sync::oneshot::Sender` so the async tauri handler can `.await` the
//! response without blocking the async runtime.
//!
//! Heartbeat
//! ─────────
//! rust_cast's synchronous channel methods internally consume messages that
//! don't match the expected response (e.g. PING) and place them in an internal
//! buffer.  Between commands the actor waits on `cmd_rx.recv_timeout(4 s)`.
//! On timeout it sends a preemptive PONG to keep the Chromecast connection
//! alive even when no commands are in flight.

use std::time::Duration;

use rust_cast::{
    CastDevice,
    channels::{
        media::{Media, Metadata, MusicTrackMediaMetadata, ResumeState, StreamType},
        receiver::CastDeviceApp,
    },
};
use serde::{Deserialize, Serialize};
use tauri::State;

use crate::AppState;

// ── Device info (shared with frontend) ───────────────────────────────────────

#[derive(Serialize, Deserialize, Clone, Debug)]
#[serde(rename_all = "camelCase")]
pub struct CastDeviceInfo {
    pub name: String,
    pub addr: String,
    pub port: u16,
}

// ── Playback status (returned by cast_get_status) ────────────────────────────

#[derive(Serialize, Deserialize, Clone, Debug)]
#[serde(rename_all = "camelCase")]
pub struct CastPlaybackStatus {
    pub current_time: f64,
    pub player_state: String,
    pub volume_level: f64,
}

// ── Actor command/response types ─────────────────────────────────────────────

type Resp<T> = tokio::sync::oneshot::Sender<Result<T, String>>;

pub enum CastActorCmd {
    Play {
        stream_url: String,
        title:      String,
        artist:     String,
        cover_url:  String,
        resp:       Resp<()>,
    },
    Pause  { resp: Resp<()> },
    Resume { resp: Resp<()> },
    Seek   { position: f64,  resp: Resp<()> },
    Volume { level: f64,     resp: Resp<()> },
    Status { resp: Resp<CastPlaybackStatus> },
    Stop   { resp: Resp<()> },
}

// ── Handle stored in AppState ─────────────────────────────────────────────────

pub struct CastActorHandle {
    pub tx:          std::sync::mpsc::Sender<CastActorCmd>,
    pub device_name: String,
    pub device_addr: String,
    pub device_port: u16,
}

// ── Actor internal state ──────────────────────────────────────────────────────

struct CastActor {
    device:           CastDevice<'static>,
    transport_id:     String,
    session_id:       String,
    media_session_id: i32,   // 0 = no active media
}

impl CastActor {
    // ── Start: connect, launch Default Media Receiver, spawn actor thread ───

    /// Connect to the Chromecast, ensure the Default Media Receiver app is
    /// running, then spin up the actor thread.  Returns a handle that callers
    /// use to send commands.
    pub fn start(
        addr: String,
        port: u16,
        name: String,
    ) -> Result<CastActorHandle, String> {
        // rustls 0.23 requires an explicit process-level crypto provider when
        // multiple backends (ring, aws-lc-rs) are present.  Ignore the error
        // if one is already installed (e.g. from a previous session).
        let _ = rustls::crypto::ring::default_provider().install_default();

        let device =
            CastDevice::connect_without_host_verification(addr.clone(), port)
                .map_err(|e| format!("Cast connect: {e}"))?;

        // Open the virtual connection to the receiver platform.
        device
            .connection
            .connect("receiver-0".to_string())
            .map_err(|e| format!("Cast CONNECT: {e}"))?;

        // Launch the Default Media Receiver app (or reuse it if already running).
        let app = device
            .receiver
            .launch_app(&CastDeviceApp::DefaultMediaReceiver)
            .map_err(|e| format!("Cast LAUNCH: {e}"))?;

        let transport_id = app.transport_id.clone();
        let session_id   = app.session_id.clone();

        // Open the virtual connection to the media transport.
        device
            .connection
            .connect(transport_id.clone())
            .map_err(|e| format!("Cast CONNECT transport: {e}"))?;

        let (tx, rx) = std::sync::mpsc::channel::<CastActorCmd>();

        let actor = CastActor {
            device,
            transport_id,
            session_id,
            media_session_id: 0,
        };

        std::thread::spawn(move || actor.run(rx));

        Ok(CastActorHandle { tx, device_name: name, device_addr: addr, device_port: port })
    }

    // ── Actor run loop ───────────────────────────────────────────────────────

    fn run(mut self, rx: std::sync::mpsc::Receiver<CastActorCmd>) {
        use std::sync::mpsc::RecvTimeoutError;

        loop {
            match rx.recv_timeout(Duration::from_secs(4)) {
                Ok(cmd) => {
                    if !self.handle(cmd) {
                        break;
                    }
                }
                Err(RecvTimeoutError::Timeout) => {
                    // Send a preemptive PONG while idle so the Chromecast
                    // doesn't close the connection due to missed heartbeats.
                    self.device.heartbeat.pong().ok();
                }
                Err(RecvTimeoutError::Disconnected) => break,
            }
        }
    }

    /// Dispatch one command.  Returns `false` when the actor should exit.
    fn handle(&mut self, cmd: CastActorCmd) -> bool {
        match cmd {
            CastActorCmd::Play { stream_url, title, artist, cover_url, resp } => {
                resp.send(self.cmd_play(stream_url, title, artist, cover_url)).ok();
            }
            CastActorCmd::Pause  { resp } => { resp.send(self.cmd_pause()).ok();  }
            CastActorCmd::Resume { resp } => { resp.send(self.cmd_resume()).ok(); }
            CastActorCmd::Seek   { position, resp } => { resp.send(self.cmd_seek(position)).ok(); }
            CastActorCmd::Volume { level,    resp } => { resp.send(self.cmd_volume(level)).ok();  }
            CastActorCmd::Status { resp } => { resp.send(self.cmd_status()).ok(); }
            CastActorCmd::Stop   { resp } => {
                resp.send(self.cmd_stop()).ok();
                return false;
            }
        }
        true
    }

    // ── Command implementations ──────────────────────────────────────────────

    fn cmd_play(
        &mut self,
        stream_url: String,
        title:      String,
        artist:     String,
        cover_url:  String,
    ) -> Result<(), String> {
        // Reset media session – we're starting a new track.
        self.media_session_id = 0;

        let images = if cover_url.is_empty() {
            vec![]
        } else {
            vec![rust_cast::channels::media::Image { url: cover_url, dimensions: None }]
        };
        let metadata = Metadata::MusicTrack(MusicTrackMediaMetadata {
            title:        Some(title),
            artist:       Some(artist),
            album_name:   None,
            album_artist: None,
            composer:     None,
            track_number: None,
            disc_number:  None,
            images,
            release_date: None,
        });

        let media = Media {
            content_id:   stream_url,
            stream_type:  StreamType::Buffered,
            content_type: "audio/mpeg".to_string(),
            metadata:     Some(metadata),
            duration:     None,
        };

        let status = self
            .device
            .media
            .load(
                self.transport_id.clone(),
                self.session_id.clone(),
                &media,
            )
            .map_err(|e| format!("Cast LOAD: {e}"))?;

        // Store the media_session_id for subsequent pause/seek/status calls.
        if let Some(entry) = status.entries.first() {
            self.media_session_id = entry.media_session_id;
        }

        Ok(())
    }

    fn cmd_pause(&mut self) -> Result<(), String> {
        if self.media_session_id == 0 {
            return Err("No active media session".into());
        }
        self.device
            .media
            .pause(self.transport_id.clone(), self.media_session_id)
            .map(|_| ())
            .map_err(|e| e.to_string())
    }

    fn cmd_resume(&mut self) -> Result<(), String> {
        if self.media_session_id == 0 {
            return Err("No active media session".into());
        }
        self.device
            .media
            .play(self.transport_id.clone(), self.media_session_id)
            .map(|_| ())
            .map_err(|e| e.to_string())
    }

    fn cmd_seek(&mut self, position: f64) -> Result<(), String> {
        if self.media_session_id == 0 {
            return Err("No active media session".into());
        }
        self.device
            .media
            .seek(
                self.transport_id.clone(),
                self.media_session_id,
                Some(position as f32),
                Some(ResumeState::PlaybackStart),
            )
            .map(|_| ())
            .map_err(|e| e.to_string())
    }

    fn cmd_volume(&mut self, level: f64) -> Result<(), String> {
        self.device
            .receiver
            .set_volume(level as f32)
            .map(|_| ())
            .map_err(|e| e.to_string())
    }

    fn cmd_status(&mut self) -> Result<CastPlaybackStatus, String> {
        // Receiver status (volume).
        let recv = self
            .device
            .receiver
            .get_status()
            .map_err(|e| e.to_string())?;
        let volume_level = recv.volume.level.unwrap_or(0.0) as f64;

        if self.media_session_id == 0 {
            return Ok(CastPlaybackStatus {
                current_time: 0.0,
                player_state: "IDLE".into(),
                volume_level,
            });
        }

        // Media status (player state + current time).
        let media = self
            .device
            .media
            .get_status(self.transport_id.clone(), Some(self.media_session_id))
            .map_err(|e| e.to_string())?;

        if let Some(entry) = media.entries.first() {
            use rust_cast::channels::media::PlayerState;
            let state = match entry.player_state {
                PlayerState::Playing   => "PLAYING",
                PlayerState::Paused    => "PAUSED",
                PlayerState::Buffering => "BUFFERING",
                PlayerState::Idle      => "IDLE",
            };

            // Update stored media_session_id in case it changed (new load).
            self.media_session_id = entry.media_session_id;

            Ok(CastPlaybackStatus {
                current_time: entry.current_time.unwrap_or(0.0) as f64,
                player_state: state.into(),
                volume_level,
            })
        } else {
            Ok(CastPlaybackStatus {
                current_time: 0.0,
                player_state: "IDLE".into(),
                volume_level,
            })
        }
    }

    fn cmd_stop(&mut self) -> Result<(), String> {
        self.device
            .receiver
            .stop_app(self.session_id.clone())
            .map_err(|e| e.to_string())
    }
}

// ── Helpers shared by tauri commands ─────────────────────────────────────────

/// Send a command to the actor and await its response.
/// Clears the actor handle from state if the channel is dead.
macro_rules! actor_send {
    ($state:expr, $variant:expr) => {{
        let tx = {
            let guard = $state.cast_actor.lock().unwrap();
            guard
                .as_ref()
                .ok_or_else(|| "No active cast session".to_string())?
                .tx
                .clone()
        };
        let (resp_tx, resp_rx) = tokio::sync::oneshot::channel();
        tx.send($variant(resp_tx))
            .map_err(|_| "Cast actor stopped".to_string())?;
        resp_rx.await.map_err(|_| "Cast actor dropped".to_string())?
    }};
}

// ── mDNS discovery ────────────────────────────────────────────────────────────
//
// Raw UDP implementation — no external mDNS crate needed.
//
// Chromecast devices respond to a PTR query for _googlecast._tcp.local. with
// a single UDP packet that already contains SRV + TXT + A records in the
// additional section.  We parse that packet directly so we get every device in
// one round-trip without a second A-record lookup.
//
// We set the unicast-response (QU) bit in the query so the Chromecast replies
// unicast to our ephemeral port instead of multicast to 224.0.0.251:5353.
// This lets us bind to any port and avoids conflicts with the Windows DNS
// Client service that already occupies port 5353.

fn mdns_query(service: &str) -> Vec<u8> {
    let mut pkt: Vec<u8> = vec![
        0x00, 0x00, // Transaction ID
        0x00, 0x00, // Flags: standard query
        0x00, 0x01, // QDCOUNT = 1
        0x00, 0x00, // ANCOUNT = 0
        0x00, 0x00, // NSCOUNT = 0
        0x00, 0x00, // ARCOUNT = 0
    ];
    for label in service.trim_end_matches('.').split('.') {
        pkt.push(label.len() as u8);
        pkt.extend_from_slice(label.as_bytes());
    }
    pkt.push(0x00);
    pkt.extend_from_slice(&[0x00, 0x0C]); // QTYPE = PTR
    pkt.extend_from_slice(&[0x80, 0x01]); // QCLASS = IN | QU bit
    pkt
}

fn dns_read_u16(buf: &[u8], pos: usize) -> Option<u16> {
    buf.get(pos..pos + 2).map(|b| u16::from_be_bytes([b[0], b[1]]))
}

fn dns_name(buf: &[u8], pos: usize) -> Option<(String, usize)> {
    let mut labels = Vec::<String>::new();
    let mut i      = pos;
    let mut jumped = false;
    let mut end    = pos;
    loop {
        if i >= buf.len() { return None; }
        let b = buf[i] as usize;
        if b == 0 {
            if !jumped { end = i + 1; }
            break;
        }
        if (b & 0xC0) == 0xC0 {
            if i + 1 >= buf.len() { return None; }
            let ptr = ((b & 0x3F) << 8) | buf[i + 1] as usize;
            if !jumped { end = i + 2; }
            jumped = true;
            i = ptr;
            continue;
        }
        i += 1;
        if i + b > buf.len() { return None; }
        labels.push(std::str::from_utf8(&buf[i..i + b]).ok()?.to_string());
        i += b;
    }
    Some((labels.join("."), end))
}

fn dns_skip_name(buf: &[u8], pos: usize) -> Option<usize> {
    dns_name(buf, pos).map(|(_, e)| e)
}

fn parse_mdns_response(buf: &[u8], sender: std::net::Ipv4Addr) -> Vec<(String, String, u16)> {
    use std::collections::HashMap;
    use std::net::Ipv4Addr;

    if buf.len() < 12 { return vec![]; }
    let qdcount = dns_read_u16(buf, 4).unwrap_or(0) as usize;
    let ancount = dns_read_u16(buf, 6).unwrap_or(0) as usize;
    let arcount = dns_read_u16(buf, 10).unwrap_or(0) as usize;

    let mut pos = 12;
    for _ in 0..qdcount {
        pos = match dns_skip_name(buf, pos) { Some(p) => p + 4, None => return vec![] };
    }

    let mut txt: HashMap<String, String>         = HashMap::new();
    let mut srv: HashMap<String, (u16, String)>  = HashMap::new();
    let mut a:   HashMap<String, Ipv4Addr>       = HashMap::new();

    for _ in 0..ancount + arcount {
        if pos >= buf.len() { break; }
        let (rname, next) = match dns_name(buf, pos) { Some(v) => v, None => break };
        pos = next;
        if pos + 10 > buf.len() { break; }
        let rtype = dns_read_u16(buf, pos).unwrap_or(0);
        let rdlen = dns_read_u16(buf, pos + 8).unwrap_or(0) as usize;
        let rdata = pos + 10;
        pos = rdata + rdlen;
        if rdata + rdlen > buf.len() { break; }

        match rtype {
            1 /* A */ => {
                if rdlen == 4 {
                    a.insert(rname, Ipv4Addr::new(buf[rdata], buf[rdata+1], buf[rdata+2], buf[rdata+3]));
                }
            }
            16 /* TXT */ => {
                let mut i = rdata;
                while i < rdata + rdlen {
                    let slen = buf[i] as usize;
                    i += 1;
                    if i + slen > rdata + rdlen { break; }
                    if let Ok(s) = std::str::from_utf8(&buf[i..i + slen]) {
                        if let Some(v) = s.strip_prefix("fn=") {
                            txt.insert(rname.clone(), v.to_string());
                        }
                    }
                    i += slen;
                }
            }
            33 /* SRV */ => {
                if rdlen >= 6 {
                    let port = dns_read_u16(buf, rdata + 4).unwrap_or(8009);
                    if let Some((target, _)) = dns_name(buf, rdata + 6) {
                        srv.insert(rname, (port, target));
                    }
                }
            }
            _ => {}
        }
    }

    let mut results = Vec::new();
    for (instance, (port, target)) in &srv {
        let ip   = a.get(target).copied().unwrap_or(sender);
        let name = txt.get(instance).cloned()
            .unwrap_or_else(|| instance.split('.').next().unwrap_or("Chromecast").to_string());
        results.push((name, ip.to_string(), *port));
    }
    results
}

#[tauri::command]
pub async fn cast_discover() -> Result<Vec<CastDeviceInfo>, String> {
    use std::collections::HashMap;
    use std::net::{IpAddr, Ipv4Addr, SocketAddr};
    use std::sync::{Arc, Mutex as StdMutex};
    use tokio::net::UdpSocket;

    const MDNS_GROUP: Ipv4Addr = Ipv4Addr::new(224, 0, 0, 251);
    const MDNS_PORT:  u16      = 5353;

    let query  = Arc::new(mdns_query("_googlecast._tcp.local."));
    let target: SocketAddr = SocketAddr::new(IpAddr::V4(MDNS_GROUP), MDNS_PORT);

    // Enumerate non-loopback IPv4 interfaces.  Sending multicast on every
    // interface ensures the query reaches the Chromecast even when VPN
    // software (e.g. Tailscale) changes the default multicast route.
    let iface_ips: Vec<Ipv4Addr> = if_addrs::get_if_addrs()
        .unwrap_or_default()
        .into_iter()
        .filter_map(|iface| {
            if iface.is_loopback() { return None; }
            match iface.addr.ip() {
                IpAddr::V4(ip) => Some(ip),
                _ => None,
            }
        })
        .collect();

    let bind_ips: Vec<Ipv4Addr> = if iface_ips.is_empty() {
        vec![Ipv4Addr::UNSPECIFIED]
    } else {
        iface_ips
    };

    let devices: Arc<StdMutex<HashMap<String, CastDeviceInfo>>> =
        Arc::new(StdMutex::new(HashMap::new()));
    let mut handles = Vec::new();

    for ip in bind_ips {
        let std_sock = match std::net::UdpSocket::bind(SocketAddr::new(IpAddr::V4(ip), 0)) {
            Ok(s) => s,
            Err(_) => continue,
        };
        std_sock.join_multicast_v4(&MDNS_GROUP, &ip).ok();
        std_sock.set_nonblocking(true).ok();
        let socket = match UdpSocket::from_std(std_sock) {
            Ok(s) => s,
            Err(_) => continue,
        };
        let _ = socket.send_to(&query, target).await;

        let q    = query.clone();
        let devs = devices.clone();
        handles.push(tokio::spawn(async move {
            let mut buf      = vec![0u8; 4096];
            let deadline     = tokio::time::Instant::now() + Duration::from_secs(5);
            let mut last_send = tokio::time::Instant::now();

            loop {
                let rem = deadline.saturating_duration_since(tokio::time::Instant::now());
                if rem.is_zero() { break; }

                if last_send.elapsed() >= Duration::from_millis(1500) {
                    let _ = socket.send_to(&q, target).await;
                    last_send = tokio::time::Instant::now();
                }

                let recv_timeout = rem.min(Duration::from_millis(250));
                let Ok(Ok((len, from))) =
                    tokio::time::timeout(recv_timeout, socket.recv_from(&mut buf)).await
                else { continue; };

                let src_ip = match from.ip() { IpAddr::V4(src) => src, _ => continue };
                for (name, addr, port) in parse_mdns_response(&buf[..len], src_ip) {
                    let key = format!("{}:{}", addr, port);
                    devs.lock().unwrap()
                        .entry(key)
                        .or_insert(CastDeviceInfo { name, addr, port });
                }
            }
        }));
    }

    tokio::time::sleep(Duration::from_secs(5)).await;
    for h in handles { h.abort(); }

    let result = devices.lock().unwrap().values().cloned().collect();
    Ok(result)
}

// ── Tauri commands ────────────────────────────────────────────────────────────

/// Connect to a Chromecast without playing anything.  Launches the Default
/// Media Receiver app so subsequent `cast_play` calls start instantly.
#[tauri::command]
pub async fn cast_connect(
    state: State<'_, AppState>,
    device_name: String,
    device_addr: String,
    device_port: u16,
) -> Result<(), String> {
    // Drop any previous actor (its thread will exit when the Sender is gone).
    *state.cast_actor.lock().unwrap() = None;

    let (name, addr, port) = (device_name.clone(), device_addr.clone(), device_port);
    let handle = tokio::task::spawn_blocking(move || {
        CastActor::start(addr, port, name)
    })
    .await
    .map_err(|e| e.to_string())??;

    *state.cast_actor.lock().unwrap() = Some(handle);
    Ok(())
}

/// Connect to a Chromecast and immediately start playing the given track.
#[tauri::command]
pub async fn cast_play(
    state: State<'_, AppState>,
    device_name: String,
    device_addr: String,
    device_port: u16,
    song_id:     String,
    stream_url:  String,
    title:       String,
    artist:      String,
    cover_url:   String,
) -> Result<(), String> {
    // If this is an external song (id starts with "ext-"), materialize it first
    // so the Chromecast receives an internal stream URL it can actually reach.
    let (stream_url, title, artist, cover_url) = if song_id.starts_with("ext-") {
        let stub = crate::commands::media::Song {
            id: song_id.clone(),
            title: title.clone(),
            artist: artist.clone(),
            album: String::new(),
            album_id: String::new(),
            cover_art: String::new(),
            cover_art_url: cover_url.clone(),
            stream_url: stream_url.clone(),
            duration: 0.0,
            audio_format: None,
            bitrate_kbps: None,
        };
        let resolved = crate::commands::library::resolve_playback_song(&state, &stub).await?;
        (resolved.stream_url, resolved.title, resolved.artist, resolved.cover_art_url)
    } else {
        (stream_url, title, artist, cover_url)
    };

    // Start a fresh actor (drops the previous one if any).
    *state.cast_actor.lock().unwrap() = None;

    let (name, addr, port) = (device_name.clone(), device_addr.clone(), device_port);
    let handle = tokio::task::spawn_blocking(move || {
        CastActor::start(addr, port, name)
    })
    .await
    .map_err(|e| e.to_string())??;

    *state.cast_actor.lock().unwrap() = Some(handle);

    // Now play on the already-connected actor.
    actor_send!(state, |resp| CastActorCmd::Play {
        stream_url, title, artist, cover_url, resp
    })
}

#[tauri::command]
pub async fn cast_pause(state: State<'_, AppState>) -> Result<(), String> {
    actor_send!(state, |resp| CastActorCmd::Pause { resp })
}

#[tauri::command]
pub async fn cast_resume(state: State<'_, AppState>) -> Result<(), String> {
    actor_send!(state, |resp| CastActorCmd::Resume { resp })
}

#[tauri::command]
pub async fn cast_stop(state: State<'_, AppState>) -> Result<(), String> {
    let result = actor_send!(state, |resp| CastActorCmd::Stop { resp });
    // Actor has exited; clear the handle.
    *state.cast_actor.lock().unwrap() = None;
    result
}

#[tauri::command]
pub async fn cast_set_volume(state: State<'_, AppState>, level: f64) -> Result<(), String> {
    actor_send!(state, |resp| CastActorCmd::Volume { level, resp })
}

#[tauri::command]
pub async fn cast_seek(state: State<'_, AppState>, position: f64) -> Result<(), String> {
    actor_send!(state, |resp| CastActorCmd::Seek { position, resp })
}

#[tauri::command]
pub async fn cast_get_status(state: State<'_, AppState>) -> Result<CastPlaybackStatus, String> {
    actor_send!(state, |resp| CastActorCmd::Status { resp })
}

/// Returns the active session info for UI restoration on startup.
#[tauri::command]
pub async fn cast_get_session(
    state: State<'_, AppState>,
) -> Result<Option<CastDeviceInfo>, String> {
    let guard = state.cast_actor.lock().unwrap();
    Ok(guard.as_ref().map(|h| CastDeviceInfo {
        name: h.device_name.clone(),
        addr: h.device_addr.clone(),
        port: h.device_port,
    }))
}

//! Android MediaSession / foreground-service bridge.
//!
//! Keeps the Android notification shade and lock screen in sync with the
//! Rust playback engine by calling into `MediaPlaybackService` via JNI.
//! Notification button presses come back through the `nativeOn*` JNI exports
//! at the bottom, which reach back into the playback engine or emit Tauri events.

use jni::{
    objects::{GlobalRef, JClass, JObject, JValue},
    JavaVM,
};
use std::sync::{Mutex, OnceLock};
use tauri::{Emitter, Manager};

// ── Globals set once from JNI_OnLoad ─────────────────────────────────────────

static JAVA_VM: OnceLock<JavaVM> = OnceLock::new();
static ANDROID_APP_CTX: OnceLock<GlobalRef> = OnceLock::new();
/// Cached GlobalRef to `com.madrify.player.MediaPlaybackService` class.
/// FindClass only works for app classes on the JVM main thread; caching here
/// lets us call static methods from any Rust thread.
static MEDIA_SERVICE_CLASS: OnceLock<GlobalRef> = OnceLock::new();

// ── Tauri AppHandle – set from app setup so JNI callbacks can control playback

static APP_HANDLE: OnceLock<tauri::AppHandle<tauri::Wry>> = OnceLock::new();

// ── Per-track metadata cache (play/pause intents need it to re-send full state)

#[derive(Default, Clone)]
struct MediaState {
    title: String,
    artist: String,
    album: String,
    cover_url: String,
    duration_ms: i64,
}

fn media_state() -> &'static Mutex<MediaState> {
    static S: OnceLock<Mutex<MediaState>> = OnceLock::new();
    S.get_or_init(|| Mutex::new(MediaState::default()))
}

// ── Public init functions ─────────────────────────────────────────────────────

/// Store the JVM, Android application context, and cached MediaPlaybackService class.
/// Called once from `JNI_OnLoad` while on the main JVM thread (where FindClass works).
pub fn init(vm: JavaVM, ctx: GlobalRef, service_class: GlobalRef) {
    JAVA_VM.set(vm).ok();
    ANDROID_APP_CTX.set(ctx).ok();
    MEDIA_SERVICE_CLASS.set(service_class).ok();
}

/// Store the Tauri app handle. Called once from the Tauri `setup` closure.
pub fn set_app_handle(handle: tauri::AppHandle<tauri::Wry>) {
    APP_HANDLE.set(handle).ok();
}

// ── JNI helpers ───────────────────────────────────────────────────────────────

fn with_jni<F>(f: F)
where
    F: FnOnce(&mut jni::JNIEnv<'_>) -> Result<(), jni::errors::Error>,
{
    let vm = match JAVA_VM.get() {
        Some(v) => v,
        None => return,
    };
    let mut env = match vm.attach_current_thread() {
        Ok(e) => e,
        Err(_) => return,
    };
    if let Err(e) = f(&mut env) {
        log::warn!("android_media JNI error: {e}");
        let _ = env.exception_describe();
        let _ = env.exception_clear();
    }
}

/// Returns the cached MediaPlaybackService JClass for the current JNI env.
/// Uses the GlobalRef cached at startup so this works from any thread.
fn service_class<'a>(
    env: &jni::JNIEnv<'a>,
) -> Option<JClass<'a>> {
    MEDIA_SERVICE_CLASS.get().map(|r| {
        // SAFETY: The GlobalRef is valid for 'static and the raw pointer
        // remains live for the duration of this JNI call.
        unsafe { JClass::from_raw(r.as_raw()) }
    })
}

fn call_service_update(state: &MediaState, is_playing: bool, position_ms: i64) {
    let ctx = match ANDROID_APP_CTX.get() {
        Some(c) => c,
        None => return,
    };
    with_jni(|env| {
        let class = match service_class(env) {
            Some(c) => c,
            None => return Err(jni::errors::Error::MethodNotFound {
                name: "MediaPlaybackService".into(),
                sig: "".into(),
            }),
        };
        let j_title    = env.new_string(&state.title)?;
        let j_artist   = env.new_string(&state.artist)?;
        let j_album    = env.new_string(&state.album)?;
        let j_cover    = env.new_string(&state.cover_url)?;
        env.call_static_method(
            class,
            "update",
            "(Landroid/content/Context;\
              Ljava/lang/String;Ljava/lang/String;Ljava/lang/String;Ljava/lang/String;\
              ZJJ)V",
            &[
                JValue::Object(ctx.as_obj()),
                JValue::Object(&JObject::from(j_title)),
                JValue::Object(&JObject::from(j_artist)),
                JValue::Object(&JObject::from(j_album)),
                JValue::Object(&JObject::from(j_cover)),
                JValue::Bool(is_playing as u8),
                JValue::Long(state.duration_ms),
                JValue::Long(position_ms),
            ],
        )?;
        Ok(())
    });
}

fn call_service_set_playing(is_playing: bool, position_ms: i64) {
    let ctx = match ANDROID_APP_CTX.get() {
        Some(c) => c,
        None => return,
    };
    with_jni(|env| {
        let class = match service_class(env) {
            Some(c) => c,
            None => return Err(jni::errors::Error::MethodNotFound {
                name: "MediaPlaybackService".into(),
                sig: "".into(),
            }),
        };
        env.call_static_method(
            class,
            "setPlayingState",
            "(Landroid/content/Context;ZJ)V",
            &[
                JValue::Object(ctx.as_obj()),
                JValue::Bool(is_playing as u8),
                JValue::Long(position_ms),
            ],
        )?;
        Ok(())
    });
}

fn call_service_stop() {
    let ctx = match ANDROID_APP_CTX.get() {
        Some(c) => c,
        None => return,
    };
    with_jni(|env| {
        let class = match service_class(env) {
            Some(c) => c,
            None => return Err(jni::errors::Error::MethodNotFound {
                name: "MediaPlaybackService".into(),
                sig: "".into(),
            }),
        };
        env.call_static_method(
            class,
            "stop",
            "(Landroid/content/Context;)V",
            &[JValue::Object(ctx.as_obj())],
        )?;
        Ok(())
    });
}

// ── Public API called from playback commands ──────────────────────────────────

/// Called from `playback_load` when a new track is queued.
pub fn update_track(
    title: &str,
    artist: &str,
    album: &str,
    cover_url: &str,
    duration_secs: f64,
    is_playing: bool,
    position_secs: f64,
) {
    let state = MediaState {
        title:       title.to_string(),
        artist:      artist.to_string(),
        album:       album.to_string(),
        cover_url:   cover_url.to_string(),
        duration_ms: (duration_secs * 1000.0) as i64,
    };
    if let Ok(mut g) = media_state().lock() {
        *g = state.clone();
    }
    call_service_update(&state, is_playing, (position_secs * 1000.0) as i64);
}

/// Called from `playback_play` / `playback_pause`.
pub fn update_playback_state(is_playing: bool, position_secs: f64) {
    call_service_set_playing(is_playing, (position_secs * 1000.0) as i64);
}

/// Called from `playback_stop`.
pub fn stop() {
    call_service_stop();
}

// ── JNI exports – called from MediaPlaybackService.Callback ──────────────────

#[no_mangle]
pub extern "C" fn Java_com_madrify_player_MediaPlaybackService_nativeOnPlay(
    _env: jni::JNIEnv,
    _this: JObject,
) {
    if let Some(handle) = APP_HANDLE.get() {
        let state: tauri::State<'_, crate::AppState> = handle.state();
        if let Some(ph) = &state.playback {
            let _ = ph.play();
        }
    }
}

#[no_mangle]
pub extern "C" fn Java_com_madrify_player_MediaPlaybackService_nativeOnPause(
    _env: jni::JNIEnv,
    _this: JObject,
) {
    if let Some(handle) = APP_HANDLE.get() {
        let state: tauri::State<'_, crate::AppState> = handle.state();
        if let Some(ph) = &state.playback {
            let _ = ph.pause();
        }
    }
}

#[no_mangle]
pub extern "C" fn Java_com_madrify_player_MediaPlaybackService_nativeOnNext(
    _env: jni::JNIEnv,
    _this: JObject,
) {
    if let Some(handle) = APP_HANDLE.get() {
        let _ = handle.emit("media:next", ());
    }
}

#[no_mangle]
pub extern "C" fn Java_com_madrify_player_MediaPlaybackService_nativeOnPrevious(
    _env: jni::JNIEnv,
    _this: JObject,
) {
    if let Some(handle) = APP_HANDLE.get() {
        let _ = handle.emit("media:previous", ());
    }
}

#[no_mangle]
pub extern "C" fn Java_com_madrify_player_MediaPlaybackService_nativeOnSeek(
    _env: jni::JNIEnv,
    _this: JObject,
    position_ms: jni::sys::jlong,
) {
    if let Some(handle) = APP_HANDLE.get() {
        let state: tauri::State<'_, crate::AppState> = handle.state();
        if let Some(ph) = &state.playback {
            let _ = ph.seek(position_ms as f64 / 1000.0);
        }
    }
}

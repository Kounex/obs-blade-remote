//! State shared by the bridge's tasks and the shell (plugin / standalone).

use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex};

use serde_json::{Value, json};
use tokio::sync::{Notify, broadcast, mpsc, watch};

use crate::e2e;
use crate::obs_config::{self, WsConfig};
use crate::rewrite::PreviewCap;
use crate::state::{self, Stored};

pub const DEFAULT_RELAY: &str = "https://relay.obs-blade.com";

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Kind {
    Plugin,
    Standalone,
}

impl Kind {
    pub fn as_str(self) -> &'static str {
        match self {
            Kind::Plugin => "plugin",
            Kind::Standalone => "standalone",
        }
    }
}

#[derive(Debug, Clone)]
pub struct Config {
    pub kind: Kind,
    /// Shown in the app ("GAMING-PC"); the shell passes a nice name.
    pub name: String,
    pub relay_url: String,
    pub state_path: PathBuf,
    /// Exact obs-websocket config.json (the plugin knows it, portable
    /// installs included); otherwise the known locations are searched.
    pub obs_ws_config: Option<PathBuf>,
    pub version: String,
}

impl Config {
    pub fn new(kind: Kind) -> Self {
        Self {
            kind,
            name: machine_name(),
            relay_url: DEFAULT_RELAY.into(),
            state_path: state::default_path(),
            obs_ws_config: None,
            version: env!("CARGO_PKG_VERSION").into(),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum RelayState {
    Disabled,
    Unpaired,
    /// Standalone while the plugin in OBS holds the link.
    Deferred,
    Connecting,
    Online,
    Offline(String),
    UpdateRequired(String),
}

impl RelayState {
    fn label(&self) -> &'static str {
        match self {
            RelayState::Disabled => "disabled",
            RelayState::Unpaired => "unpaired",
            RelayState::Deferred => "deferred",
            RelayState::Connecting => "connecting",
            RelayState::Online => "online",
            RelayState::Offline(_) => "offline",
            RelayState::UpdateRequired(_) => "update_required",
        }
    }
}

pub type Sink = Arc<dyn Fn(Value) + Send + Sync>;

pub struct Shared {
    pub cfg: Config,
    stored: Mutex<Stored>,
    pub devices: Mutex<Vec<Value>>,
    pub relay_state: Mutex<RelayState>,
    pub obs_state: watch::Sender<String>,
    pub obs_running: watch::Sender<Option<bool>>,
    /// Whether obs-websocket's settings were found (or entered by hand)
    pub obs_settings_found: watch::Sender<bool>,
    pub plugin_present: watch::Sender<bool>,
    /// Standalone: the monitor answered "is the plugin loaded?" once (or
    /// found no OBS). The relay link waits for it - connecting first
    /// replaced the plugin's link at the relay (4906) on every start.
    pub plugin_checked: watch::Sender<bool>,
    pub status: watch::Sender<Option<Value>>,
    /// Stored settings / pairing changed.
    pub wake: Notify,
    /// Some(obs_exit) once the shell stops the bridge.
    pub stop: watch::Sender<Option<bool>>,
    pub control_tx: mpsc::UnboundedSender<Value>,
    pub control_rx: Mutex<Option<mpsc::UnboundedReceiver<Value>>>,
    sink: Sink,
    last_emitted: Mutex<String>,
    /// Codes already redeemed (Some(ack)) or in flight (None): the app
    /// re-sends its code until it has the bridge's ack (a CustomEvent can
    /// be missed), a redeemed code gets the ack again.
    recent_codes: Mutex<Vec<(String, Option<String>)>>,
    /// obs-websocket messages for the monitor's OBS connection to send
    /// (the pairing ack); dropped while OBS isn't connected.
    pub obs_outbox: broadcast::Sender<String>,
    /// OBS base canvas width / height (GetVideoSettings), for the preview cap.
    pub canvas_aspect: Mutex<Option<f64>>,
    /// A newer release the relay told about (`update_available`), for the
    /// panel and the tray.
    pub update_available: Mutex<Option<String>>,
}

impl Shared {
    pub fn new(cfg: Config, sink: Sink) -> Arc<Self> {
        let stored = state::load(&cfg.state_path);
        let (control_tx, control_rx) = mpsc::unbounded_channel();
        Arc::new(Self {
            cfg,
            stored: Mutex::new(stored),
            devices: Mutex::new(Vec::new()),
            relay_state: Mutex::new(RelayState::Connecting),
            obs_state: watch::Sender::new(String::new()),
            obs_running: watch::Sender::new(None),
            obs_settings_found: watch::Sender::new(true),
            plugin_present: watch::Sender::new(false),
            plugin_checked: watch::Sender::new(false),
            status: watch::Sender::new(None),
            wake: Notify::new(),
            stop: watch::Sender::new(None),
            control_tx,
            control_rx: Mutex::new(Some(control_rx)),
            sink,
            last_emitted: Mutex::new(String::new()),
            recent_codes: Mutex::new(Vec::new()),
            update_available: Mutex::new(None),
            obs_outbox: broadcast::Sender::new(16),
            canvas_aspect: Mutex::new(None),
        })
    }

    pub fn stored(&self) -> Stored {
        self.stored.lock().unwrap().clone()
    }

    /// Re-read from disk: the plugin and the standalone share the file.
    pub fn reload(&self) -> Stored {
        let fresh = state::load(&self.cfg.state_path);
        *self.stored.lock().unwrap() = fresh.clone();
        fresh
    }

    pub fn update(&self, change: impl FnOnce(&mut Stored)) {
        let snapshot = {
            let mut stored = self.stored.lock().unwrap();
            change(&mut stored);
            stored.clone()
        };
        if let Err(error) = state::save(&self.cfg.state_path, &snapshot) {
            self.log("error", format!("saving bridge state failed: {error}"));
        }
        self.wake.notify_waiters();
        self.emit_state();
    }

    /// obs-websocket's settings plus the manual override. Not found
    /// anywhere (a portable OBS before it was seen running, a custom
    /// location): OBS's default port without a password is tried, and the
    /// panel says the settings weren't found (`obs_settings_found`).
    pub fn ws_config(&self) -> WsConfig {
        let stored = self.stored();
        let found = obs_config::find(self.cfg.obs_ws_config.as_deref(), stored.obs_exe.as_deref().map(Path::new));
        let settings_found = found.is_some() || stored.obs_port.is_some() || stored.obs_password.is_some();
        if self.obs_settings_found.send_replace(settings_found) != settings_found {
            self.emit_state();
        }
        let mut ws = found
            .map(|(_, c)| c)
            .unwrap_or(WsConfig { enabled: true, auth_required: false, ..WsConfig::default() });
        if let Some(port) = stored.obs_port {
            ws.port = port;
            ws.enabled = true;
        }
        if let Some(password) = stored.obs_password {
            ws.auth_required = !password.is_empty();
            ws.password = password;
        }
        ws
    }

    pub fn preview_cap(&self) -> PreviewCap {
        let stored = self.stored();
        PreviewCap {
            max_edge: stored.preview_max_edge,
            jpeg_quality: stored.preview_jpeg_quality,
            canvas_aspect: *self.canvas_aspect.lock().unwrap(),
        }
    }

    pub fn set_relay_state(&self, state: RelayState) {
        *self.relay_state.lock().unwrap() = state;
        self.emit_state();
    }

    pub fn set_obs_state(&self, state: &str) {
        self.obs_state.send_if_modified(|current| {
            if current == state {
                return false;
            }
            *current = state.to_owned();
            true
        });
        self.emit_state();
    }

    pub fn set_devices(&self, devices: Vec<Value>) {
        *self.devices.lock().unwrap() = devices;
        self.emit_state();
    }

    pub fn emit(&self, event: Value) {
        (self.sink)(event);
    }

    /// Diagnostics for the shell's log (the plugin writes them to OBS's log).
    /// The relay's `update_available`: kept (and logged once) only when it
    /// really is newer than this build.
    pub fn offer_update(&self, version: &str) {
        if !newer_release(version, &self.cfg.version) {
            return;
        }
        {
            let mut offered = self.update_available.lock().unwrap();
            if offered.as_deref() == Some(version) {
                return;
            }
            *offered = Some(version.to_owned());
        }
        self.log("info", format!("OBS Blade Remote {version} is available (this is {})", self.cfg.version));
        self.emit_state();
    }

    pub fn log(&self, level: &str, message: impl Into<String>) {
        self.emit(json!({"type": "log", "level": level, "message": message.into()}));
    }

    /// Full state for the panel; sent only when it changed.
    pub fn emit_state(&self) {
        let stored = self.stored();
        let relay = self.relay_state.lock().unwrap().clone();
        let detail = match &relay {
            RelayState::Offline(reason) => Some(reason.clone()),
            RelayState::UpdateRequired(min) => Some(min.clone()),
            _ => None,
        };
        let event = json!({
            "type": "state",
            "kind": self.cfg.kind.as_str(),
            "relay": relay.label(),
            "detail": detail,
            "obs": *self.obs_state.borrow(),
            "paired": stored.paired(),
            "bridge_id": stored.bridge_id,
            "enabled": stored.enabled,
            "devices": *self.devices.lock().unwrap(),
            "preview_max_edge": stored.preview_max_edge,
            "preview_jpeg_quality": stored.preview_jpeg_quality,
            "obs_port": stored.obs_port,
            "obs_password_set": stored.obs_password.is_some(),
            "obs_settings_found": *self.obs_settings_found.borrow(),
            "obs_exe": stored.obs_exe,
            "bridge_key": stored.e2e_public,
            "name": self.cfg.name,
            "version": self.cfg.version,
            "update_available": *self.update_available.lock().unwrap(),
        });
        let text = event.to_string();
        {
            let mut last = self.last_emitted.lock().unwrap();
            if *last == text {
                return;
            }
            *last = text;
        }
        self.emit(event);
    }

    /// What to do with a pairing code the app sent (it repeats it).
    pub fn pair_attempt(&self, code: &str) -> PairAttempt {
        let code = normalize_code(code);
        let mut recent = self.recent_codes.lock().unwrap();
        if let Some((_, ack)) = recent.iter().find(|(c, _)| *c == code) {
            return match ack {
                Some(ack) => PairAttempt::Done(ack.clone()),
                None => PairAttempt::InFlight,
            };
        }
        if recent.len() >= 16 {
            recent.remove(0);
        }
        recent.push((code, None));
        PairAttempt::New
    }

    /// A redeem finished: [ack] (the CustomEvent to repeat), or None to
    /// let the next repeat of the code try again.
    pub fn pair_finished(&self, code: &str, ack: Option<String>) {
        let code = normalize_code(code);
        let mut recent = self.recent_codes.lock().unwrap();
        match ack {
            Some(ack) => {
                if let Some(entry) = recent.iter_mut().find(|(c, _)| *c == code) {
                    entry.1 = Some(ack);
                }
            }
            None => recent.retain(|(c, _)| *c != code),
        }
    }

    /// This bridge's end-to-end key pair, made on first use.
    pub fn e2e_keypair(&self) -> (Vec<u8>, Vec<u8>) {
        let stored = self.reload();
        if let (Some(private), Some(public)) = (
            stored.e2e_private.as_deref().and_then(e2e::key_from_b64),
            stored.e2e_public.as_deref().and_then(e2e::key_from_b64),
        ) {
            return (private, public);
        }
        let keypair = e2e::generate();
        self.update(|s| {
            s.e2e_private = Some(e2e::key_to_b64(&keypair.private));
            s.e2e_public = Some(e2e::key_to_b64(&keypair.public));
        });
        (keypair.private, keypair.public)
    }

    /// The paired phones' static keys (decoded).
    pub fn app_keys(&self) -> Vec<Vec<u8>> {
        self.reload().app_keys.iter().filter_map(|k| e2e::key_from_b64(&k.key)).collect()
    }

    pub fn relay_http(&self) -> String {
        self.cfg.relay_url.trim_end_matches('/').to_owned()
    }

    pub fn relay_ws(&self) -> String {
        let base = self.relay_http();
        let ws = if let Some(rest) = base.strip_prefix("https://") {
            format!("wss://{rest}")
        } else if let Some(rest) = base.strip_prefix("http://") {
            format!("ws://{rest}")
        } else {
            base
        };
        format!("{ws}/v1/bridge")
    }

    pub fn stopping(&self) -> bool {
        self.stop.borrow().is_some()
    }
}

/// The PC's name as people know it.
pub fn machine_name() -> String {
    #[cfg(target_os = "windows")]
    if let Ok(name) = std::env::var("COMPUTERNAME") {
        return name;
    }
    #[cfg(target_os = "macos")]
    if let Some(name) = command_output("scutil", &["--get", "ComputerName"]) {
        return name;
    }
    #[cfg(target_os = "linux")]
    if let Ok(name) = std::fs::read_to_string("/etc/hostname") {
        let name = name.trim();
        if !name.is_empty() {
            return name.to_owned();
        }
    }
    command_output("hostname", &[])
        .map(|n| n.split('.').next().unwrap_or(&n).to_owned())
        .unwrap_or_else(|| "OBS PC".into())
}

fn command_output(program: &str, args: &[&str]) -> Option<String> {
    let output = std::process::Command::new(program).args(args).output().ok()?;
    let text = String::from_utf8(output.stdout).ok()?;
    let text = text.trim();
    (output.status.success() && !text.is_empty()).then(|| text.to_owned())
}

#[derive(Debug, PartialEq, Eq)]
pub enum PairAttempt {
    New,
    InFlight,
    /// Redeemed already: the ack to broadcast again.
    Done(String),
}

fn normalize_code(code: &str) -> String {
    code.chars().filter(char::is_ascii_alphanumeric).collect::<String>().to_ascii_uppercase()
}

/// "1.2.3" as numbers; None for anything else (pre-releases included).
fn release(version: &str) -> Option<(u32, u32, u32)> {
    let mut parts = version.split('.').map(|p| p.parse::<u32>().ok());
    let release = (parts.next()??, parts.next()??, parts.next()??);
    parts.next().is_none().then_some(release)
}

/// Whether `candidate` is a release newer than `mine`.
pub fn newer_release(candidate: &str, mine: &str) -> bool {
    matches!((release(candidate), release(mine)), (Some(c), Some(m)) if c > m)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn only_a_newer_plain_release_counts() {
        assert!(newer_release("0.4.3", "0.4.2"));
        assert!(newer_release("0.10.0", "0.9.9"));
        assert!(newer_release("1.0.0", "0.99.99"));
        assert!(!newer_release("0.4.2", "0.4.2"));
        assert!(!newer_release("0.4.1", "0.4.2"));
        for odd in ["", "0.4", "0.4.3.1", "0.4.3-beta", "v0.4.3", "x.y.z"] {
            assert!(!newer_release(odd, "0.4.2"), "{odd}");
        }
        assert!(!newer_release("0.4.3", "dev"));
    }

    #[test]
    fn an_offered_update_reaches_the_state_once() {
        let events = Arc::new(Mutex::new(Vec::<Value>::new()));
        let seen = events.clone();
        let mut cfg = Config::new(Kind::Standalone);
        cfg.version = "0.4.2".into();
        cfg.state_path = std::env::temp_dir().join(format!("obb-offer-{}.json", std::process::id()));
        let shared = Shared::new(cfg, Arc::new(move |event: Value| seen.lock().unwrap().push(event)));
        let logs = |events: &[Value]| events.iter().filter(|e| e["type"] == "log").count();

        shared.offer_update("0.4.1"); // older: ignored
        shared.offer_update("0.4.2"); // the same: ignored
        assert_eq!(*shared.update_available.lock().unwrap(), None);
        assert_eq!(logs(&events.lock().unwrap()), 0);

        shared.offer_update("0.4.3");
        shared.offer_update("0.4.3"); // again (a reconnect): nothing new
        assert_eq!(shared.update_available.lock().unwrap().as_deref(), Some("0.4.3"));
        let events = events.lock().unwrap();
        assert_eq!(logs(&events), 1);
        let state = events.iter().rev().find(|e| e["type"] == "state").expect("a state event");
        assert_eq!(state["update_available"], "0.4.3");
        let _ = std::fs::remove_file(&shared.cfg.state_path);
    }
}

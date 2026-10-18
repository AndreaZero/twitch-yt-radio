//! YouTube community API. OAuth, tokens and HTTP stay in Rust; MPV is independent.
//! Reads use the documented REST polling interval. Writes are never retried.
use crate::token_store;
use base64::{engine::general_purpose::URL_SAFE_NO_PAD, Engine};
use rand::TryRngCore;
use reqwest::{Client, Method};
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use sha2::{Digest, Sha256};
use std::{
    sync::{atomic::AtomicU64, Arc, Mutex},
    time::{Duration, SystemTime, UNIX_EPOCH},
};
use tauri::async_runtime::JoinHandle;
use tauri::{AppHandle, Emitter, Manager, State};
use tokio::{
    io::{AsyncReadExt, AsyncWriteExt},
    net::TcpListener,
    sync::Mutex as AsyncMutex,
};
use url::Url;

const API: &str = "https://www.googleapis.com/youtube/v3/";
const SCOPE: &str = "https://www.googleapis.com/auth/youtube.force-ssl";
const TOKEN: &str = "https://oauth2.googleapis.com/token";

#[derive(Clone, Serialize, Deserialize)]
struct Credentials {
    client_id: String,
    client_secret: String,
    access: String,
    refresh: String,
    expires: u64,
    login: String,
}

#[derive(Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Message {
    id: String,
    user_id: String,
    name: String,
    text: String,
    color: String,
    published_at: String,
    like_count: u64,
    reply_count: u64,
}

#[derive(Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Snapshot {
    version: u64,
    login: Option<String>,
    can_send: bool,
    detached: bool,
    channel: Option<String>, // selected video ID, shared by both windows
    status: String,
    error: String,
    mode: String,
    can_load_more: bool,
    loading: bool,
    messages: Vec<Message>,
    moderation: Vec<Delta>,
    user_code: Option<String>,
    verification_uri: Option<String>,
}

#[derive(Clone, Serialize)]
#[serde(rename_all = "camelCase")]
struct Delta {
    version: u64,
    kind: String,
    target: String,
}

pub struct Youtube {
    state: Snapshot,
    credentials: Option<Credentials>,
    epoch: u64,
    auth_epoch: u64,
    task: Option<JoinHandle<()>>,
    auth_task: Option<JoinHandle<()>>,
    generation: Arc<AtomicU64>,
    refresh_lock: Arc<AsyncMutex<()>>,
    send_lock: Arc<AsyncMutex<()>>,
    read_lock: Arc<AsyncMutex<()>>,
    live_id: String,
    video_channel: String,
    next_page: String,
    seen: std::collections::VecDeque<String>,
    last_send: Option<std::time::Instant>,
    persistence_error: String,
    next_live_read: std::time::Instant,
    poll_interval: Duration,
}
impl Default for Youtube {
    fn default() -> Self {
        Self {
            state: Snapshot {
                version: 0,
                login: None,
                can_send: false,
                detached: false,
                channel: None,
                status: "Collega Google nelle impostazioni".into(),
                error: String::new(),
                mode: "idle".into(),
                can_load_more: false,
                loading: false,
                messages: vec![],
                moderation: vec![],
                user_code: None,
                verification_uri: None,
            },
            credentials: None,
            epoch: 0,
            auth_epoch: 0,
            task: None,
            auth_task: None,
            generation: Arc::new(AtomicU64::new(0)),
            refresh_lock: Arc::new(AsyncMutex::new(())),
            send_lock: Arc::new(AsyncMutex::new(())),
            read_lock: Arc::new(AsyncMutex::new(())),
            live_id: String::new(),
            video_channel: String::new(),
            next_page: String::new(),
            seen: Default::default(),
            last_send: None,
            persistence_error: String::new(),
            next_live_read: std::time::Instant::now(),
            poll_interval: Duration::from_secs(5),
        }
    }
}
pub type SharedYoutube = Arc<Mutex<Youtube>>;

fn emit(app: &AppHandle, yt: &mut Youtube) {
    yt.state.version += 1;
    let _ = app.emit("youtube-state", snapshot(yt));
}
fn snapshot(yt: &Youtube) -> Snapshot {
    let mut state = yt.state.clone();
    if !yt.persistence_error.is_empty() {
        if !state.error.is_empty() {
            state.error.push_str(" · ");
        }
        state.error.push_str(&yt.persistence_error);
    }
    state
}
fn now() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs()
}
fn http() -> Result<Client, String> {
    Client::builder()
        .timeout(Duration::from_secs(25))
        .connect_timeout(Duration::from_secs(10))
        .redirect(reqwest::redirect::Policy::none())
        .build()
        .map_err(|_| "Client Google non disponibile".into())
}

// Never accept an arbitrary API endpoint or callback URL from the frontend.
fn video_id(value: &str) -> Result<String, String> {
    if value.len() > 2048 {
        return Err("Link YouTube troppo lungo".into());
    }
    let url = Url::parse(value).map_err(|_| "Link YouTube non valido")?;
    if url.scheme() != "https"
        || !url.username().is_empty()
        || url.password().is_some()
        || url.port().is_some()
    {
        return Err("Usa un link HTTPS YouTube senza credenziali o porte".into());
    }
    let segments: Vec<_> = url.path().split('/').filter(|s| !s.is_empty()).collect();
    let id = match url.host_str().unwrap_or("") {
        "youtu.be" if segments.len() == 1 => Some(segments[0].to_string()),
        "youtube.com" | "www.youtube.com" | "m.youtube.com" | "music.youtube.com" => {
            if url.path() == "/watch" {
                url.query_pairs()
                    .find(|(k, _)| k == "v")
                    .map(|(_, v)| v.into_owned())
            } else if segments.len() == 2 && matches!(segments[0], "live" | "shorts" | "embed") {
                Some(segments[1].to_string())
            } else {
                None
            }
        }
        _ => None,
    }
    .ok_or("Per chat e commenti serve il link di un video, non di un canale YouTube")?;
    if id.len() != 11
        || !id
            .bytes()
            .all(|b| b.is_ascii_alphanumeric() || b == b'_' || b == b'-')
    {
        return Err("ID video YouTube non valido".into());
    }
    Ok(id)
}

#[derive(Debug)]
struct ApiError {
    message: String,
    retry: bool,
}
impl From<String> for ApiError {
    fn from(message: String) -> Self {
        Self {
            message,
            retry: false,
        }
    }
}
fn network() -> ApiError {
    ApiError {
        message: "Connessione a YouTube interrotta. Verifica la rete".into(),
        retry: true,
    }
}
async fn response(response: reqwest::Response) -> Result<Value, ApiError> {
    let status = response.status();
    let value: Value = response.json().await.map_err(|_| network())?;
    if status.is_success() {
        return Ok(value);
    }
    let reason = value["error"]["errors"][0]["reason"].as_str().unwrap_or("");
    let message = match reason {
        "quotaExceeded" | "dailyLimitExceeded" => "Quota YouTube del progetto esaurita. Attendi il ripristino della quota in Google Cloud",
        "accessNotConfigured" => "Abilita YouTube Data API v3 nel progetto Google Cloud",
        "commentsDisabled" => "Il proprietario ha disattivato i commenti per questo video",
        "liveChatDisabled" => "La chat di questa diretta è disattivata",
        "liveChatEnded" => "La diretta è terminata: la chat non è più disponibile",
        "forbidden" | "insufficientPermissions" => "YouTube non consente questa operazione. Verifica account, consenso e restrizioni del canale",
        "rateLimitExceeded" | "userRequestsExceedRateLimit" => "YouTube richiede una pausa. Attendi prima di riprovare",
        "youtubeSignupRequired" | "channelNotFound" => "Crea prima un canale YouTube con questo account Google, poi ricollegalo",
        "ineligibleToParticipate" => "Questo account non può partecipare alla chat. Verifica le restrizioni della diretta",
        "messageTextInvalid" | "commentTextTooLong" | "commentTextRequired" | "commentTextInvalid" => "YouTube ha rifiutato il testo. Modificalo prima di inviare di nuovo",
        "videoNotFound" | "notFound" | "liveChatNotFound" => "Video o chat YouTube non disponibili per questo account",
        _ if status.as_u16() == 401 => "Accesso Google scaduto o revocato. Ricollega l’account nelle impostazioni",
        _ if status.is_server_error() => "YouTube non risponde. Riprovo tra poco",
        _ => "Richiesta rifiutata da YouTube. Verifica API, account e permessi del video",
    };
    Err(ApiError {
        message: message.into(),
        retry: status.is_server_error(),
    })
}

// A refresh runs independently of watch-task cancellation and serializes token
// rotation. The vault generation prevents logout from resurrecting an account.
async fn access(shared: SharedYoutube) -> Result<Credentials, ApiError> {
    tauri::async_runtime::spawn(async move {
        let lock = shared
            .lock()
            .map_err(|_| "Account Google occupato".to_string())?
            .refresh_lock
            .clone();
        let _guard = lock.lock().await;
        let (mut credentials, epoch, generation) = {
            let yt = shared
                .lock()
                .map_err(|_| "Account Google occupato".to_string())?;
            (
                yt.credentials
                    .clone()
                    .ok_or("Collega Google nelle impostazioni".to_string())?,
                yt.auth_epoch,
                yt.generation.clone(),
            )
        };
        if credentials.expires > now() + 90 {
            return Ok(credentials);
        }
        let response = http()?
            .post(TOKEN)
            .form(&[
                ("client_id", credentials.client_id.as_str()),
                ("client_secret", &credentials.client_secret),
                ("refresh_token", &credentials.refresh),
                ("grant_type", "refresh_token"),
            ])
            .send()
            .await
            .map_err(|_| network())?;
        let status = response.status();
        let token: Value = response.json().await.map_err(|_| network())?;
        if !status.is_success() {
            let permanent = matches!(
                token["error"].as_str(),
                Some("invalid_grant" | "invalid_client" | "unauthorized_client")
            );
            if permanent {
                let mut yt = shared
                    .lock()
                    .map_err(|_| "Account Google occupato".to_string())?;
                if yt.auth_epoch == epoch {
                    yt.state.can_send = false;
                    // Keep the account label so settings exposes reconnect; no endless refresh loop.
                    yt.credentials = None;
                    token_store::clear_for(token_store::GOOGLE, &generation, epoch)
                        .map_err(ApiError::from)?;
                }
            }
            return Err(ApiError {
                message: if permanent {
                    "Accesso Google revocato o scaduto. Ricollega l’account"
                } else {
                    "Rinnovo Google non disponibile. Verifica la rete e riprova"
                }
                .into(),
                retry: !permanent && status.is_server_error(),
            });
        }
        credentials.access = token["access_token"]
            .as_str()
            .ok_or("Token Google non valido".to_string())?
            .into();
        if let Some(refresh) = token["refresh_token"].as_str() {
            credentials.refresh = refresh.into();
        }
        credentials.expires = now() + token["expires_in"].as_u64().unwrap_or(3600);
        let mut yt = shared
            .lock()
            .map_err(|_| "Account Google occupato".to_string())?;
        if yt.auth_epoch != epoch || yt.credentials.is_none() {
            return Err("Account Google cambiato".to_string().into());
        }
        // Save a rotated refresh token before any subsequent await.
        let saved = token_store::save_for(token_store::GOOGLE, &credentials, &generation, epoch);
        yt.credentials = Some(credentials.clone());
        yt.persistence_error = saved.err().unwrap_or_default();
        Ok(credentials)
    })
    .await
    .map_err(|_| ApiError::from("Rinnovo Google interrotto".to_string()))?
}

async fn get(
    shared: &SharedYoutube,
    path: &str,
    params: &[(&str, &str)],
) -> Result<Value, ApiError> {
    let credentials = access(shared.clone()).await?;
    response(
        http()?
            .get(format!("{API}{path}"))
            .bearer_auth(&credentials.access)
            .query(params)
            .send()
            .await
            .map_err(|_| network())?,
    )
    .await
}

fn random() -> Result<String, String> {
    let mut bytes = [0u8; 32];
    rand::rngs::OsRng
        .try_fill_bytes(&mut bytes)
        .map_err(|_| "Generatore sicuro Windows non disponibile")?;
    Ok(URL_SAFE_NO_PAD.encode(bytes))
}

#[cfg(windows)]
fn open_browser(url: &Url) -> Result<(), String> {
    use windows_sys::Win32::UI::Shell::ShellExecuteW;
    let value: Vec<u16> = url.as_str().encode_utf16().chain(Some(0)).collect();
    let verb: Vec<u16> = "open".encode_utf16().chain(Some(0)).collect();
    if unsafe {
        ShellExecuteW(
            std::ptr::null_mut(),
            verb.as_ptr(),
            value.as_ptr(),
            std::ptr::null(),
            std::ptr::null(),
            1,
        )
    } as isize
        <= 32
    {
        return Err("Impossibile aprire il browser per Google".into());
    }
    Ok(())
}
#[cfg(not(windows))]
fn open_browser(_: &Url) -> Result<(), String> {
    Err("Accesso Google disponibile su Windows".into())
}

// Loopback only, ephemeral port, one random state, PKCE S256, bounded headers
// and deadline. Never echo callback query parameters or credentials in HTML.
async fn callback(listener: TcpListener, expected: &str) -> Result<String, String> {
    loop {
        let (mut socket, _) = listener
            .accept()
            .await
            .map_err(|_| "Callback Google non disponibile")?;
        let request = tokio::time::timeout(Duration::from_secs(3), async {
            let mut bytes = Vec::new();
            let mut chunk = [0u8; 1024];
            while bytes.len() < 8192 {
                let count = socket.read(&mut chunk).await.map_err(|_| ())?;
                if count == 0 {
                    break;
                }
                bytes.extend_from_slice(&chunk[..count]);
                if bytes.windows(4).any(|w| w == b"\r\n\r\n") {
                    break;
                }
            }
            String::from_utf8(bytes).map_err(|_| ())
        })
        .await;
        let Ok(Ok(request)) = request else {
            continue;
        };
        if request.len() > 8192 || !request.contains("\r\n\r\n") {
            continue;
        }
        let target = request
            .lines()
            .next()
            .and_then(|line| line.strip_prefix("GET "))
            .and_then(|line| line.split_whitespace().next());
        let url = target
            .filter(|s| s.starts_with("/?"))
            .and_then(|s| Url::parse(&format!("http://127.0.0.1{s}")).ok());
        let valid = url.as_ref().is_some_and(|url| {
            url.query_pairs().filter(|(k, _)| k == "state").count() == 1
                && url
                    .query_pairs()
                    .any(|(k, v)| k == "state" && v == expected)
        });
        let body = if valid {
            "Autorizzazione ricevuta. Torna a Background Audio per completare l'accesso."
        } else {
            "Richiesta non valida. Torna all'app e riprova."
        };
        let reply = format!("HTTP/1.1 {}\r\nContent-Type: text/plain; charset=utf-8\r\nCache-Control: no-store\r\nContent-Security-Policy: default-src 'none'\r\nConnection: close\r\nContent-Length: {}\r\n\r\n{body}", if valid { "200 OK" } else { "400 Bad Request" }, body.len());
        let _ =
            tokio::time::timeout(Duration::from_secs(2), socket.write_all(reply.as_bytes())).await;
        if !valid {
            continue;
        }
        let url = url.unwrap();
        if url.query_pairs().any(|(k, _)| k == "error") {
            return Err("Accesso Google annullato o negato nel browser".into());
        }
        return url
            .query_pairs()
            .find(|(k, _)| k == "code")
            .map(|(_, v)| v.into_owned())
            .filter(|v| !v.is_empty())
            .ok_or("Codice Google assente".into());
    }
}

async fn authorize(client_id: String, client_secret: String) -> Result<Credentials, String> {
    let listener = TcpListener::bind((std::net::Ipv4Addr::LOCALHOST, 0))
        .await
        .map_err(|_| "Impossibile aprire il callback locale Google")?;
    let redirect = format!(
        "http://127.0.0.1:{}/",
        listener
            .local_addr()
            .map_err(|_| "Porta Google non disponibile")?
            .port()
    );
    let verifier = random()?;
    let state = random()?;
    let challenge = URL_SAFE_NO_PAD.encode(Sha256::digest(verifier.as_bytes()));
    let mut url = Url::parse("https://accounts.google.com/o/oauth2/v2/auth").unwrap();
    url.query_pairs_mut().extend_pairs([
        ("client_id", client_id.as_str()),
        ("redirect_uri", &redirect),
        ("response_type", "code"),
        ("scope", SCOPE),
        ("state", &state),
        ("code_challenge", &challenge),
        ("code_challenge_method", "S256"),
        ("access_type", "offline"),
        ("prompt", "consent select_account"),
    ]);
    open_browser(&url)?;
    let code = tokio::time::timeout(Duration::from_secs(180), callback(listener, &state))
        .await
        .map_err(|_| "Accesso Google scaduto dopo 3 minuti. Riprova")??;
    let response = http()?
        .post(TOKEN)
        .form(&[
            ("client_id", client_id.as_str()),
            ("client_secret", &client_secret),
            ("code", &code),
            ("code_verifier", &verifier),
            ("redirect_uri", &redirect),
            ("grant_type", "authorization_code"),
        ])
        .send()
        .await
        .map_err(|_| "Scambio del codice Google interrotto. Riprova l’accesso")?;
    let status = response.status();
    let token: Value = response
        .json()
        .await
        .map_err(|_| "Risposta di accesso Google non valida")?;
    if !status.is_success() {
        return Err(
            "Google ha rifiutato l’accesso. Verifica Client ID e Client Secret di tipo Desktop"
                .into(),
        );
    }
    if !token["scope"]
        .as_str()
        .unwrap_or("")
        .split_whitespace()
        .any(|s| s == SCOPE)
    {
        return Err(
            "Il consenso YouTube di lettura e scrittura non è stato concesso. Ricollega Google"
                .into(),
        );
    }
    let access = token["access_token"]
        .as_str()
        .ok_or("Token Google assente")?
        .to_string();
    let refresh = token["refresh_token"]
        .as_str()
        .ok_or("Google non ha fornito un accesso offline. Riprova autorizzando il consenso")?
        .to_string();
    let profile = response_profile(&access).await?;
    Ok(Credentials {
        client_id,
        client_secret,
        access,
        refresh,
        login: profile,
        expires: now() + token["expires_in"].as_u64().unwrap_or(3600),
    })
}

async fn response_profile(access: &str) -> Result<String, String> {
    let value = response(
        http()?
            .get(format!("{API}channels"))
            .bearer_auth(access)
            .query(&[("part", "snippet"), ("mine", "true")])
            .send()
            .await
            .map_err(|_| "Profilo YouTube non raggiungibile")?,
    )
    .await
    .map_err(|e| e.message)?;
    value["items"][0]["snippet"]["title"]
        .as_str()
        .map(str::to_owned)
        .ok_or(
            "Questo account non ha un canale YouTube. Crealo su YouTube, poi ricollega Google"
                .into(),
        )
}

#[tauri::command]
pub fn youtube_get_state(shared: State<'_, SharedYoutube>) -> Result<Snapshot, String> {
    let yt = shared.lock().map_err(|_| "YouTube occupato")?;
    Ok(snapshot(&yt))
}

#[tauri::command]
pub fn youtube_login(
    app: AppHandle,
    shared: State<'_, SharedYoutube>,
    client_id: String,
    client_secret: String,
) -> Result<(), String> {
    let client_id = client_id.trim().to_owned();
    let client_secret = client_secret.trim().to_owned();
    if !client_id.ends_with(".apps.googleusercontent.com")
        || client_id.len() > 256
        || client_id
            .bytes()
            .any(|b| !(b.is_ascii_alphanumeric() || b".-".contains(&b)))
        || client_secret.is_empty()
        || client_secret.len() > 256
        || client_secret.chars().any(char::is_control)
    {
        return Err(
            "Inserisci Client ID e Client Secret OAuth di tipo App desktop da Google Cloud".into(),
        );
    }
    youtube_logout(app.clone(), shared.clone())?;
    let shared = shared.inner().clone();
    let mut yt = shared.lock().map_err(|_| "YouTube occupato")?;
    let epoch = yt.auth_epoch;
    yt.state.status = "Accesso Google in corso".into();
    yt.state.loading = true;
    yt.state.error.clear();
    emit(&app, &mut yt);
    let target = shared.clone();
    yt.auth_task = Some(tauri::async_runtime::spawn(async move {
        let result = authorize(client_id, client_secret).await;
        let success = {
            let Ok(mut yt) = target.lock() else {
                return;
            };
            if yt.auth_epoch != epoch {
                return;
            }
            yt.state.loading = false;
            match result {
                Ok(credentials) => {
                    let saved = token_store::save_for(
                        token_store::GOOGLE,
                        &credentials,
                        &yt.generation,
                        epoch,
                    );
                    yt.state.login = Some(credentials.login.clone());
                    yt.credentials = Some(credentials);
                    yt.state.status = "Account Google collegato".into();
                    yt.persistence_error = saved.err().unwrap_or_default();
                    emit(&app, &mut yt);
                    true
                }
                Err(error) => {
                    yt.state.status = "Collega Google nelle impostazioni".into();
                    yt.state.error = error;
                    emit(&app, &mut yt);
                    false
                }
            }
        };
        if success {
            restart(&app, &target);
        }
    }));
    Ok(())
}

#[tauri::command]
pub fn youtube_logout(app: AppHandle, shared: State<'_, SharedYoutube>) -> Result<(), String> {
    let mut yt = shared.lock().map_err(|_| "YouTube occupato")?;
    yt.auth_epoch += 1;
    let saved = token_store::clear_for(token_store::GOOGLE, &yt.generation, yt.auth_epoch);
    yt.epoch += 1;
    if let Some(task) = yt.auth_task.take() {
        task.abort();
    }
    if let Some(task) = yt.task.take() {
        task.abort();
    }
    yt.credentials = None;
    yt.state.login = None;
    yt.state.can_send = false;
    yt.state.loading = false;
    yt.state.can_load_more = false;
    yt.next_page.clear();
    yt.live_id.clear();
    yt.video_channel.clear();
    yt.persistence_error.clear();
    yt.state.mode = "idle".into();
    yt.state.status = "Collega Google nelle impostazioni".into();
    yt.state.messages.clear();
    yt.state.moderation.clear();
    yt.seen.clear();
    yt.state.error = saved.as_ref().err().cloned().unwrap_or_default();
    emit(&app, &mut yt);
    saved
}

pub fn restore(app: &AppHandle) {
    let shared = app.state::<SharedYoutube>().inner().clone();
    let mut yt = match shared.lock() {
        Ok(yt) => yt,
        Err(_) => return,
    };
    match token_store::read_for::<Credentials>(token_store::GOOGLE) {
        Ok(Some(credentials)) => {
            yt.state.login = Some(credentials.login.clone());
            yt.state.status = "Account Google ricordato".into();
            yt.credentials = Some(credentials);
        }
        Ok(None) => (),
        Err(error) => yt.state.error = error,
    }
    emit(app, &mut yt);
    // Refresh lazily when a YouTube source is selected, without waking playback.
}

#[tauri::command]
pub fn youtube_watch(
    app: AppHandle,
    shared: State<'_, SharedYoutube>,
    url: Option<String>,
) -> Result<(), String> {
    let id = url.as_deref().map(video_id).transpose();
    let mut yt = shared.lock().map_err(|_| "YouTube occupato")?;
    let id = match id {
        Ok(id) => id,
        Err(error) => {
            yt.epoch += 1;
            if let Some(task) = yt.task.take() {
                task.abort();
            }
            yt.state.channel = None;
            yt.state.messages.clear();
            yt.state.moderation.clear();
            yt.state.can_send = false;
            yt.state.loading = false;
            yt.state.can_load_more = false;
            yt.state.mode = "unavailable".into();
            yt.state.status = error.clone();
            emit(&app, &mut yt);
            return Err(error);
        }
    };
    if yt.state.channel == id {
        return Ok(());
    }
    yt.state.channel = id;
    drop(yt);
    restart(&app, shared.inner());
    Ok(())
}

fn restart(app: &AppHandle, shared: &SharedYoutube) {
    let Ok(mut yt) = shared.lock() else {
        return;
    };
    yt.epoch += 1;
    if let Some(task) = yt.task.take() {
        task.abort();
    }
    yt.state.messages.clear();
    yt.state.moderation.clear();
    yt.seen.clear();
    yt.state.can_send = false;
    yt.state.can_load_more = false;
    yt.state.mode = "idle".into();
    yt.state.loading = false;
    yt.state.error.clear();
    yt.next_page.clear();
    yt.live_id.clear();
    yt.video_channel.clear();
    let Some(id) = yt.state.channel.clone() else {
        yt.state.status = "Avvia un video YouTube nel player".into();
        emit(app, &mut yt);
        return;
    };
    if yt.credentials.is_none() {
        yt.state.status = "Collega Google nelle impostazioni".into();
        emit(app, &mut yt);
        return;
    }
    yt.state.loading = true;
    yt.state.error.clear();
    yt.state.status = "Connessione a YouTube…".into();
    let epoch = yt.epoch;
    emit(app, &mut yt);
    let app = app.clone();
    let target = shared.clone();
    yt.task = Some(tauri::async_runtime::spawn(async move {
        let mut delay = 2;
        loop {
            match watch(&app, &target, epoch, &id).await {
                Ok(()) => break,
                Err(error) => {
                    {
                        let Ok(mut yt) = target.lock() else {
                            return;
                        };
                        if yt.epoch != epoch {
                            return;
                        }
                        yt.state.loading = false;
                        yt.state.can_send = false;
                        yt.state.error = error.message;
                        yt.state.status = if error.retry {
                            "Riconnessione YouTube…"
                        } else {
                            "YouTube non disponibile"
                        }
                        .into();
                        emit(&app, &mut yt);
                    }
                    if !error.retry {
                        break;
                    }
                    tokio::time::sleep(Duration::from_secs(delay)).await;
                    delay = (delay * 2).min(30);
                }
            }
        }
    }));
}

#[tauri::command]
pub fn youtube_retry(app: AppHandle, shared: State<'_, SharedYoutube>) {
    restart(&app, shared.inner());
}

async fn watch(
    app: &AppHandle,
    shared: &SharedYoutube,
    epoch: u64,
    id: &str,
) -> Result<(), ApiError> {
    let value = get(
        shared,
        "videos",
        &[("part", "snippet,liveStreamingDetails"), ("id", id)],
    )
    .await?;
    let video = value["items"]
        .as_array()
        .and_then(|v| v.first())
        .ok_or("Video YouTube non disponibile".to_string())?;
    let live_id = video["liveStreamingDetails"]["activeLiveChatId"]
        .as_str()
        .unwrap_or("")
        .to_string();
    let broadcast = video["snippet"]["liveBroadcastContent"]
        .as_str()
        .unwrap_or("none");
    {
        let mut yt = shared.lock().map_err(|_| "YouTube occupato".to_string())?;
        if yt.epoch != epoch {
            return Ok(());
        }
        yt.video_channel = video["snippet"]["channelId"].as_str().unwrap_or("").into();
        yt.state.error.clear();
        if !live_id.is_empty() {
            // Retain continuation across transient read failures.
            if yt.live_id != live_id {
                yt.next_page.clear();
            }
            yt.live_id = live_id.clone();
            yt.state.mode = "live".into();
        } else if broadcast == "live" || broadcast == "upcoming" {
            yt.state.mode = "unavailable".into();
            yt.state.loading = false;
            yt.state.status = if broadcast == "upcoming" {
                "Diretta programmata · chat non ancora disponibile"
            } else {
                "Diretta senza chat disponibile"
            }
            .into();
            emit(app, &mut yt);
        } else {
            yt.state.mode = "comments".into();
            yt.state.status = "Commenti del video".into();
            yt.next_page.clear();
            emit(app, &mut yt);
        }
    }
    if live_id.is_empty() {
        if broadcast == "live" || broadcast == "upcoming" {
            tokio::time::sleep(Duration::from_secs(30)).await;
            return Err(ApiError {
                message: "".into(),
                retry: true,
            });
        }
        return comments(app, shared, epoch, id, false).await;
    }
    loop {
        let (page, wait) = {
            let yt = shared.lock().map_err(|_| "YouTube occupato".to_string())?;
            if yt.epoch != epoch {
                return Ok(());
            }
            (
                yt.next_page.clone(),
                yt.next_live_read
                    .saturating_duration_since(std::time::Instant::now()),
            )
        };
        tokio::time::sleep(wait).await;
        {
            let mut yt = shared.lock().map_err(|_| "YouTube occupato".to_string())?;
            if yt.epoch != epoch {
                return Ok(());
            }
            // Preserve the API cooldown across failures, retries and video switches.
            yt.next_live_read = std::time::Instant::now() + yt.poll_interval;
        }
        let mut params = vec![
            ("part", "id,snippet,authorDetails"),
            ("liveChatId", &live_id),
            ("maxResults", "200"),
        ];
        if !page.is_empty() {
            params.push(("pageToken", &page));
        }
        let value = get(shared, "liveChat/messages", &params).await?;
        let interval = value["pollingIntervalMillis"]
            .as_u64()
            .unwrap_or(5000)
            .max(5000);
        let mut ended = value["offlineAt"].as_str().is_some();
        {
            let mut yt = shared.lock().map_err(|_| "YouTube occupato".to_string())?;
            if yt.epoch != epoch {
                return Ok(());
            }
            yt.state.loading = false;
            yt.poll_interval = Duration::from_millis(interval);
            yt.next_live_read = std::time::Instant::now() + yt.poll_interval;
            yt.state.error.clear();
            yt.state.status = if ended {
                "Diretta terminata"
            } else {
                "In diretta"
            }
            .into();
            yt.state.can_send = !ended && yt.credentials.is_some();
            yt.next_page = value["nextPageToken"].as_str().unwrap_or("").into();
            if let Some(items) = value["items"].as_array() {
                for item in items {
                    apply_live(&mut yt, item);
                }
            }
            ended |= yt.state.status == "Diretta terminata";
            emit(app, &mut yt);
        }
        if ended {
            return Ok(());
        }
        tokio::time::sleep(Duration::from_millis(interval)).await;
    }
}

fn apply_live(yt: &mut Youtube, item: &Value) {
    let Some(id) = item["id"].as_str() else {
        return;
    };
    if item["snippet"]["type"] != "tombstone" && yt.seen.iter().any(|seen| seen == id) {
        return;
    }
    yt.seen.push_back(id.into());
    while yt.seen.len() > 1000 {
        yt.seen.pop_front();
    }
    let snippet = &item["snippet"];
    let (kind, target) = match snippet["type"].as_str().unwrap_or("") {
        "messageDeletedEvent" => (
            "delete",
            snippet["messageDeletedDetails"]["deletedMessageId"]
                .as_str()
                .unwrap_or(""),
        ),
        "userBannedEvent" => (
            "clearUser",
            snippet["userBannedDetails"]["bannedUserDetails"]["channelId"]
                .as_str()
                .unwrap_or(""),
        ),
        "chatEndedEvent" => {
            yt.state.can_send = false;
            yt.state.status = "Diretta terminata".into();
            return;
        }
        "tombstone" => ("delete", id),
        _ => ("", ""),
    };
    if !kind.is_empty() {
        yt.state.messages.retain(|message| {
            if kind == "delete" {
                message.id != target
            } else {
                message.user_id != target
            }
        });
        // Emit version will be the next snapshot version; frozen UIs still apply removals.
        yt.state.moderation.push(Delta {
            version: yt.state.version + 1,
            kind: kind.into(),
            target: target.into(),
        });
        if yt.state.moderation.len() > 1000 {
            yt.state.moderation.remove(0);
        }
        return;
    }
    let Some(text) = snippet["displayMessage"].as_str() else {
        return;
    };
    yt.state.messages.push(Message {
        id: id.into(),
        user_id: item["authorDetails"]["channelId"]
            .as_str()
            .unwrap_or("")
            .into(),
        name: item["authorDetails"]["displayName"]
            .as_str()
            .unwrap_or("YouTube")
            .into(),
        text: text.into(),
        color: "".into(),
        published_at: snippet["publishedAt"].as_str().unwrap_or("").into(),
        like_count: 0,
        reply_count: 0,
    });
    if yt.state.messages.len() > 200 {
        yt.state.messages.remove(0);
    }
}

fn comment(item: &Value, replies: u64) -> Option<Message> {
    let snippet = &item["snippet"];
    Some(Message {
        id: item["id"].as_str()?.into(),
        user_id: snippet["authorChannelId"]["value"]
            .as_str()
            .unwrap_or("")
            .into(),
        name: snippet["authorDisplayName"]
            .as_str()
            .unwrap_or("YouTube")
            .into(),
        text: snippet["textOriginal"]
            .as_str()
            .or_else(|| snippet["textDisplay"].as_str())
            .unwrap_or("")
            .into(),
        color: "".into(),
        published_at: snippet["publishedAt"].as_str().unwrap_or("").into(),
        like_count: snippet["likeCount"].as_u64().unwrap_or(0),
        reply_count: replies,
    })
}

async fn comments(
    app: &AppHandle,
    shared: &SharedYoutube,
    epoch: u64,
    id: &str,
    more: bool,
) -> Result<(), ApiError> {
    let lock = shared
        .lock()
        .map_err(|_| "YouTube occupato".to_string())?
        .read_lock
        .clone();
    let _guard = lock.lock().await;
    let page = {
        let yt = shared.lock().map_err(|_| "YouTube occupato".to_string())?;
        if yt.epoch != epoch || yt.state.mode != "comments" {
            return Ok(());
        }
        if more && yt.next_page.is_empty() {
            return Ok(());
        }
        yt.next_page.clone()
    };
    let mut params = vec![
        ("part", "snippet"),
        ("videoId", id),
        ("textFormat", "plainText"),
        ("order", "time"),
        ("maxResults", "50"),
    ];
    if more {
        params.push(("pageToken", &page));
    }
    let value = get(shared, "commentThreads", &params).await?;
    let rows: Vec<_> = value["items"]
        .as_array()
        .into_iter()
        .flatten()
        .filter_map(|thread| {
            comment(
                &thread["snippet"]["topLevelComment"],
                thread["snippet"]["totalReplyCount"].as_u64().unwrap_or(0),
            )
        })
        .collect();
    let mut yt = shared.lock().map_err(|_| "YouTube occupato".to_string())?;
    if yt.epoch != epoch {
        return Ok(());
    }
    if !more {
        yt.state.messages.clear();
    }
    for row in rows {
        if !yt.state.messages.iter().any(|old| old.id == row.id) {
            yt.state.messages.push(row);
        }
    }
    yt.state.messages.truncate(1000);
    yt.next_page = value["nextPageToken"].as_str().unwrap_or("").into();
    yt.state.can_load_more = !yt.next_page.is_empty() && yt.state.messages.len() < 1000;
    yt.state.loading = false;
    yt.state.can_send = yt.credentials.is_some();
    yt.state.status = "Commenti del video · più recenti".into();
    yt.state.error.clear();
    emit(app, &mut yt);
    Ok(())
}

#[tauri::command]
pub async fn youtube_more_comments(
    app: AppHandle,
    shared: State<'_, SharedYoutube>,
) -> Result<(), String> {
    let (epoch, id) = {
        let mut yt = shared.lock().map_err(|_| "YouTube occupato")?;
        if yt.state.mode != "comments" || !yt.state.can_load_more || yt.state.loading {
            return Ok(());
        }
        yt.state.loading = true;
        let data = (yt.epoch, yt.state.channel.clone().unwrap_or_default());
        emit(&app, &mut yt);
        data
    };
    let result = comments(&app, shared.inner(), epoch, &id, true).await;
    if let Err(error) = &result {
        let mut yt = shared.lock().map_err(|_| "YouTube occupato")?;
        if yt.epoch == epoch {
            yt.state.loading = false;
            yt.state.error = error.message.clone();
            emit(&app, &mut yt);
        }
    }
    result.map_err(|error| error.message)
}

#[tauri::command]
pub async fn youtube_send_message(
    app: AppHandle,
    shared: State<'_, SharedYoutube>,
    channel: String,
    message: String,
) -> Result<(), String> {
    let message = message.trim();
    let (epoch, auth_epoch, mode, live_id, video_channel, lock) = {
        let yt = shared.lock().map_err(|_| "YouTube occupato")?;
        if yt.state.channel.as_deref() != Some(channel.as_str()) || !yt.state.can_send {
            return Err("Video o account cambiato. Attendi la connessione".into());
        }
        (
            yt.epoch,
            yt.auth_epoch,
            yt.state.mode.clone(),
            yt.live_id.clone(),
            yt.video_channel.clone(),
            yt.send_lock.clone(),
        )
    };
    let _guard = lock
        .try_lock()
        .map_err(|_| "Invio già in corso. Attendi prima di inviare di nuovo")?;
    let limit = if mode == "live" {
        200
    } else if mode == "comments" {
        10000
    } else {
        return Err("Invio non disponibile".into());
    };
    if message.is_empty()
        || message.chars().count() > limit
        || message
            .chars()
            .any(|c| c.is_control() && c != '\n' && c != '\t')
    {
        return Err(format!(
            "Inserisci un testo valido di massimo {limit} caratteri"
        ));
    }
    let credentials = access(shared.inner().clone())
        .await
        .map_err(|error| error.message)?;
    {
        let mut yt = shared.lock().map_err(|_| "YouTube occupato")?;
        if yt.epoch != epoch || yt.auth_epoch != auth_epoch || !yt.state.can_send {
            return Err("Video o account cambiato: messaggio non inviato".into());
        }
        if yt
            .last_send
            .is_some_and(|time| time.elapsed() < Duration::from_secs(2))
        {
            return Err("Attendi due secondi prima di inviare di nuovo".into());
        }
        yt.last_send = Some(std::time::Instant::now());
    }
    let (path, body) = if mode == "live" {
        (
            "liveChat/messages",
            json!({"snippet":{"liveChatId":live_id,"type":"textMessageEvent","textMessageDetails":{"messageText":message}}}),
        )
    } else {
        (
            "commentThreads",
            json!({"snippet":{"channelId":video_channel,"videoId":channel,"topLevelComment":{"snippet":{"textOriginal":message}}}}),
        )
    };
    // An ambiguous timeout may mean the message was published. Never auto-retry.
    let result = http()?.request(Method::POST, format!("{API}{path}")).bearer_auth(&credentials.access)
        .query(&[("part", "snippet")]).json(&body).send().await
        .map_err(|_| "Invio interrotto: il messaggio potrebbe essere stato pubblicato. Controlla prima di riprovare")?;
    let value = response(result).await.map_err(|error| if error.retry {
        "Esito dell’invio non confermato. Il messaggio potrebbe essere pubblicato: controlla prima di riprovare".to_string()
    } else { error.message })?;
    if mode == "comments" {
        let mut yt = shared.lock().map_err(|_| "YouTube occupato")?;
        if yt.epoch == epoch && yt.auth_epoch == auth_epoch {
            if let Some(row) = comment(&value["snippet"]["topLevelComment"], 0) {
                yt.state.messages.insert(0, row);
                yt.state.messages.truncate(1000);
                emit(&app, &mut yt);
            }
        }
    }
    Ok(())
}

#[tauri::command]
pub fn youtube_set_detached(
    app: AppHandle,
    shared: State<'_, SharedYoutube>,
    detached: bool,
) -> Result<(), String> {
    if !detached {
        return close_detached(&app);
    }
    let window = app
        .get_webview_window("youtube-chat")
        .ok_or("Finestra YouTube non disponibile")?;
    window
        .show()
        .map_err(|_| "Impossibile aprire la finestra YouTube")?;
    window
        .set_focus()
        .map_err(|_| "Impossibile attivare la finestra YouTube")?;
    let mut yt = shared.lock().map_err(|_| "YouTube occupato")?;
    yt.state.detached = true;
    emit(&app, &mut yt);
    Ok(())
}
pub fn close_detached(app: &AppHandle) -> Result<(), String> {
    if let Some(window) = app.get_webview_window("youtube-chat") {
        window
            .hide()
            .map_err(|_| "Impossibile reinserire il pannello YouTube")?;
    }
    let shared = app.state::<SharedYoutube>();
    let mut yt = shared.lock().map_err(|_| "YouTube occupato")?;
    yt.state.detached = false;
    emit(app, &mut yt);
    if let Some(window) = app.get_webview_window("main") {
        let _ = window.show();
        let _ = window.set_focus();
    }
    Ok(())
}
#[tauri::command]
pub fn youtube_open_console() -> Result<(), String> {
    open_browser(&Url::parse("https://console.cloud.google.com/apis/credentials").unwrap())
}

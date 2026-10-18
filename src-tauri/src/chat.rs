//! Twitch EventSub and message sending. Credentials stay in Rust memory.
use futures_util::{SinkExt, StreamExt};
use reqwest::Client;
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use std::{
    collections::VecDeque,
    sync::{Arc, Mutex},
    time::{Duration, Instant},
};
use tauri::{AppHandle, Emitter, State};
use tokio_tungstenite::{connect_async, tungstenite::Message};

const WS: &str = "wss://eventsub.wss.twitch.tv/ws";
const READ_SCOPE: &str = "user:read:chat";
const WRITE_SCOPE: &str = "user:write:chat";
const SCOPE: &str = "user:read:chat user:write:chat";
const LIMIT: usize = 200;
pub type SharedChat = Arc<Mutex<Chat>>;

#[derive(Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ChatMessage {
    id: String,
    user_id: String,
    name: String,
    text: String,
    color: String,
}

#[derive(Clone, Default, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ChatSnapshot {
    version: u64,
    login: Option<String>,
    can_send: bool,
    channel: Option<String>,
    status: String,
    error: String,
    user_code: Option<String>,
    verification_uri: Option<String>,
    messages: VecDeque<ChatMessage>,
}

#[derive(Clone)]
struct Credentials {
    client_id: String,
    access: String,
    refresh: String,
    user_id: String,
    can_write: bool,
}
type SharedCredentials = Arc<tokio::sync::Mutex<Credentials>>;

#[derive(Default)]
pub struct Chat {
    snapshot: ChatSnapshot,
    credentials: Option<SharedCredentials>,
    auth_task: Option<tauri::async_runtime::JoinHandle<()>>,
    chat_task: Option<tauri::async_runtime::JoinHandle<()>>,
    auth_epoch: u64,
    chat_epoch: u64,
    seen: VecDeque<String>,
    broadcaster_id: Option<String>,
    send_lock: Arc<tokio::sync::Mutex<()>>,
    last_send: Option<Instant>,
}

fn client() -> Client {
    Client::builder()
        .timeout(Duration::from_secs(15))
        .redirect(reqwest::redirect::Policy::none())
        .build()
        .expect("Client TLS Twitch")
}

fn announce(app: &AppHandle, chat: &mut Chat) {
    chat.snapshot.version += 1;
    let _ = app.emit("twitch-chat-state", &chat.snapshot);
}

fn status(app: &AppHandle, shared: &SharedChat, epoch: u64, text: &str, error: &str) {
    if let Ok(mut c) = shared.lock() {
        if c.chat_epoch != epoch {
            return;
        }
        c.snapshot.status = text.into();
        c.snapshot.error = error.into();
        announce(app, &mut c);
    }
}

#[tauri::command]
pub fn chat_get_state(state: State<'_, SharedChat>) -> Result<ChatSnapshot, String> {
    Ok(state
        .lock()
        .map_err(|_| "Chat non disponibile")?
        .snapshot
        .clone())
}

#[derive(Deserialize)]
struct Device {
    device_code: String,
    user_code: String,
    verification_uri: String,
    interval: u64,
    expires_in: u64,
}
#[derive(Deserialize)]
struct Tokens {
    access_token: String,
    refresh_token: String,
}

async fn validate(http: &Client, auth: &Credentials) -> Result<Value, SessionError> {
    let response = http
        .get("https://id.twitch.tv/oauth2/validate")
        .header("Authorization", format!("OAuth {}", auth.access))
        .send()
        .await
        .map_err(|_| {
            (
                false,
                "Impossibile verificare l'accesso Twitch. Controlla la connessione".into(),
            )
        })?;
    if !response.status().is_success() {
        return Err((
            response.status().as_u16() == 401,
            "Verifica Twitch non riuscita: accesso scaduto o servizio non disponibile".into(),
        ));
    }
    let v: Value = response
        .json()
        .await
        .map_err(|_| (false, "Risposta Twitch non valida".into()))?;
    if v["client_id"].as_str() != Some(&auth.client_id)
        || !v["scopes"]
            .as_array()
            .is_some_and(|s| s.iter().any(|s| s == READ_SCOPE))
    {
        return Err((
            true,
            "Accesso Twitch privo del permesso di lettura chat".into(),
        ));
    }
    Ok(v)
}

#[tauri::command]
pub fn chat_login(
    client_id: String,
    app: AppHandle,
    state: State<'_, SharedChat>,
) -> Result<(), String> {
    let client_id = client_id.trim().to_owned();
    if !(8..=128).contains(&client_id.len())
        || !client_id.bytes().all(|b| b.is_ascii_alphanumeric())
    {
        return Err("Inserisci il Client ID della tua app Twitch pubblica".into());
    }
    let shared = state.inner().clone();
    let mut c = shared.lock().map_err(|_| "Chat non disponibile")?;
    if let Some(task) = c.auth_task.take() {
        task.abort();
    }
    if let Some(task) = c.chat_task.take() {
        task.abort();
    }
    c.auth_epoch += 1;
    c.chat_epoch += 1;
    c.credentials = None;
    c.snapshot.login = None;
    c.snapshot.can_send = false;
    c.broadcaster_id = None;
    c.snapshot.messages.clear();
    c.snapshot.user_code = None;
    c.snapshot.verification_uri = None;
    c.snapshot.status = "Accesso in corso".into();
    c.snapshot.error.clear();
    announce(&app, &mut c);
    let epoch = c.auth_epoch;
    let worker = shared.clone();
    c.auth_task = Some(tauri::async_runtime::spawn(async move {
        if let Err(error) = authorize(&app, &worker, epoch, client_id).await {
            if let Ok(mut c) = worker.lock() {
                if c.auth_epoch == epoch {
                    c.snapshot.status = "Account scollegato".into();
                    c.snapshot.error = error;
                    c.snapshot.user_code = None;
                    c.snapshot.verification_uri = None;
                    announce(&app, &mut c);
                }
            }
        }
    }));
    Ok(())
}

async fn authorize(
    app: &AppHandle,
    shared: &SharedChat,
    epoch: u64,
    client_id: String,
) -> Result<(), String> {
    let http = client();
    let response = http
        .post("https://id.twitch.tv/oauth2/device")
        .form(&[("client_id", client_id.as_str()), ("scopes", SCOPE)])
        .send()
        .await
        .map_err(|_| "Twitch non raggiungibile. Controlla la connessione")?;
    if !response.status().is_success() {
        return Err("Client ID rifiutato. Verifica che l'app Twitch sia di tipo Public".into());
    }
    let device: Device = response
        .json()
        .await
        .map_err(|_| "Codice Twitch non valido")?;
    activation_url(&device.verification_uri)?;
    {
        let mut c = shared.lock().map_err(|_| "Chat non disponibile")?;
        if c.auth_epoch != epoch {
            return Ok(());
        }
        c.snapshot.user_code = Some(device.user_code);
        c.snapshot.verification_uri = Some(device.verification_uri);
        c.snapshot.status = "Autorizza nel browser".into();
        announce(app, &mut c);
    }
    let deadline = Instant::now() + Duration::from_secs(device.expires_in.min(1800));
    let mut interval = device.interval.max(5);
    while Instant::now() < deadline {
        tokio::time::sleep(Duration::from_secs(interval)).await;
        let response = http
            .post("https://id.twitch.tv/oauth2/token")
            .form(&[
                ("client_id", client_id.as_str()),
                ("scopes", SCOPE),
                ("device_code", device.device_code.as_str()),
                ("grant_type", "urn:ietf:params:oauth:grant-type:device_code"),
            ])
            .send()
            .await
            .map_err(|_| "Connessione Twitch interrotta durante l'accesso")?;
        let success = response.status().is_success();
        let value: Value = response
            .json()
            .await
            .map_err(|_| "Risposta Twitch non valida")?;
        if !success {
            match value["message"]
                .as_str()
                .or(value["error"].as_str())
                .unwrap_or("")
            {
                "authorization_pending" => continue,
                "slow_down" => {
                    interval += 5;
                    continue;
                }
                _ => {
                    return Err(
                        "Accesso negato o codice scaduto. Premi Collega Twitch per riprovare"
                            .into(),
                    )
                }
            }
        }
        let tokens: Tokens =
            serde_json::from_value(value).map_err(|_| "Token Twitch non valido")?;
        let mut auth = Credentials {
            client_id,
            access: tokens.access_token,
            refresh: tokens.refresh_token,
            user_id: String::new(),
            can_write: false,
        };
        let identity = validate(&http, &auth).await.map_err(|(_, e)| e)?;
        auth.can_write = identity["scopes"].as_array().is_some_and(|s| s.iter().any(|s| s == WRITE_SCOPE));
        auth.user_id = identity["user_id"]
            .as_str()
            .ok_or("Account Twitch non valido")?
            .into();
        {
            let mut c = shared.lock().map_err(|_| "Chat non disponibile")?;
            if c.auth_epoch != epoch {
                return Ok(());
            }
            c.snapshot.can_send = auth.can_write;
            c.credentials = Some(Arc::new(tokio::sync::Mutex::new(auth)));
            c.snapshot.login = identity["login"].as_str().map(str::to_owned);
            c.snapshot.user_code = None;
            c.snapshot.verification_uri = None;
            c.snapshot.error.clear();
            restart(app, shared, &mut c);
        }
        return Ok(());
    }
    Err("Codice Twitch scaduto. Collega di nuovo l'account".into())
}

fn restart(app: &AppHandle, shared: &SharedChat, c: &mut Chat) {
    if let Some(task) = c.chat_task.take() {
        task.abort();
    }
    c.chat_epoch += 1;
    c.broadcaster_id = None;
    c.seen.clear();
    let epoch = c.chat_epoch;
    if let (Some(auth), Some(channel)) = (c.credentials.clone(), c.snapshot.channel.clone()) {
        c.snapshot.status = "Connessione chat…".into();
        let app = app.clone();
        let shared = shared.clone();
        c.chat_task = Some(tauri::async_runtime::spawn(async move {
            run_chat(app, shared, epoch, channel, auth).await;
        }));
    } else {
        c.snapshot.status = if c.credentials.is_none() {
            "Collega Twitch nelle impostazioni"
        } else {
            "Avvia un canale Twitch"
        }
        .into();
    }
    announce(app, c);
}

fn valid_channel(channel: &str) -> bool {
    (1..=25).contains(&channel.len())
        && channel
            .bytes()
            .all(|b| b.is_ascii_alphanumeric() || b == b'_')
}

#[tauri::command]
pub fn chat_watch(
    channel: Option<String>,
    app: AppHandle,
    state: State<'_, SharedChat>,
) -> Result<(), String> {
    let channel = channel.map(|s| s.to_ascii_lowercase());
    if channel.as_ref().is_some_and(|s| !valid_channel(s)) {
        return Err("Nome canale Twitch non valido".into());
    }
    let mut c = state.lock().map_err(|_| "Chat non disponibile")?;
    if c.snapshot.channel == channel {
        return Ok(());
    }
    c.snapshot.channel = channel;
    c.snapshot.messages.clear();
    c.snapshot.error.clear();
    restart(&app, state.inner(), &mut c);
    Ok(())
}

#[tauri::command]
pub fn chat_retry(app: AppHandle, state: State<'_, SharedChat>) -> Result<(), String> {
    let mut c = state.lock().map_err(|_| "Chat non disponibile")?;
    c.snapshot.error.clear();
    restart(&app, state.inner(), &mut c);
    Ok(())
}

#[tauri::command]
pub fn chat_logout(app: AppHandle, state: State<'_, SharedChat>) -> Result<(), String> {
    let mut c = state.lock().map_err(|_| "Chat non disponibile")?;
    if let Some(task) = c.auth_task.take() {
        task.abort();
    }
    c.auth_epoch += 1;
    c.credentials = None;
    c.snapshot.login = None;
    c.snapshot.can_send = false;
    c.snapshot.messages.clear();
    c.snapshot.user_code = None;
    c.snapshot.verification_uri = None;
    c.snapshot.error.clear();
    restart(&app, state.inner(), &mut c);
    Ok(())
}

fn validate_outgoing_message(message: &str) -> Result<&str, String> {
    let message = message.trim();
    if message.is_empty() { return Err("Scrivi un messaggio prima di inviare".into()); }
    if message.chars().count() > 500 { return Err("Il messaggio supera i 500 caratteri consentiti da Twitch".into()); }
    if message.chars().any(char::is_control) { return Err("Il messaggio deve essere su una sola riga e senza caratteri di controllo".into()); }
    Ok(message)
}

#[tauri::command]
pub async fn chat_send_message(channel: String, message: String, state: State<'_, SharedChat>) -> Result<(), String> {
    let message = validate_outgoing_message(&message)?.to_owned();
    let channel = channel.to_ascii_lowercase();
    if !valid_channel(&channel) { return Err("Canale Twitch non valido".into()); }
    let shared = state.inner().clone();
    let (credentials, broadcaster, epoch, send_lock) = {
        let c = shared.lock().map_err(|_| "Chat non disponibile")?;
        if c.snapshot.channel.as_deref() != Some(channel.as_str()) || c.snapshot.status != "In diretta" {
            return Err("Attendi la connessione alla chat del canale corrente".into());
        }
        if !c.snapshot.can_send { return Err("Ricollega Twitch nelle impostazioni per autorizzare l'invio dei messaggi".into()); }
        (c.credentials.clone().ok_or("Collega prima il tuo account Twitch")?,
            c.broadcaster_id.clone().ok_or("Canale chat non ancora pronto")?, c.chat_epoch, c.send_lock.clone())
    };
    let _permit = send_lock.try_lock_owned().map_err(|_| "Un messaggio è già in invio. Attendi".to_string())?;
    let http = client();
    let auth = prepare_credentials(&http, &credentials).await.map_err(|(_, e)| e)?;
    if !auth.can_write { return Err("Manca il permesso user:write:chat. Ricollega Twitch nelle impostazioni".into()); }
    {
        let mut c = shared.lock().map_err(|_| "Chat non disponibile")?;
        // Never redirect a pending send to a channel/account selected in the meantime.
        if c.chat_epoch != epoch || c.snapshot.channel.as_deref() != Some(channel.as_str()) || c.snapshot.status != "In diretta" {
            return Err("Invio annullato: canale o account cambiato".into());
        }
        if c.last_send.is_some_and(|at| at.elapsed() < Duration::from_secs(1)) { return Err("Attendi un secondo prima di inviare un altro messaggio".into()); }
        c.last_send = Some(Instant::now());
    }
    // A failed/ambiguous POST is never retried automatically: that could duplicate a message.
    let response = http.post("https://api.twitch.tv/helix/chat/messages")
        .bearer_auth(&auth.access).header("Client-Id", &auth.client_id)
        .json(&json!({"broadcaster_id":broadcaster,"sender_id":auth.user_id,"message":message}))
        .send().await.map_err(|_| "Esito invio non confermato. Controlla la chat prima di riprovare")?;
    match response.status().as_u16() {
        200 => {},
        401 => return Err("Accesso scaduto o permesso di scrittura mancante. Ricollega Twitch".into()),
        403 => return Err("Twitch non consente di scrivere in questa chat. Verifica ban, timeout o restrizioni dell'account".into()),
        429 => return Err("Limite Twitch raggiunto. Attendi prima di inviare di nuovo".into()),
        n if n >= 500 => return Err("Esito invio non confermato da Twitch. Controlla la chat prima di riprovare".into()),
        n => return Err(format!("Messaggio rifiutato da Twitch (HTTP {n}). Verifica testo e regole del canale")),
    }
    let value: Value = response.json().await.map_err(|_| "Risposta di invio non leggibile. Controlla la chat prima di riprovare")?;
    let result = &value["data"][0];
    if result["is_sent"].as_bool() != Some(true) {
        let reason: String = result["drop_reason"]["message"].as_str().unwrap_or("Controlla AutoMod e le regole del canale").chars().take(300).collect();
        return Err(format!("Messaggio non pubblicato: {reason}"));
    }
    // EventSub supplies our own message, so the UI does not create a second local copy.
    Ok(())
}

fn activation_url(value: &str) -> Result<(), String> {
    let url = url::Url::parse(value).map_err(|_| "Link Twitch non valido")?;
    if url.scheme() != "https"
        || url.host_str() != Some("www.twitch.tv")
        || url.path() != "/activate"
        || !url.username().is_empty()
        || url.password().is_some()
        || url.port().is_some()
    {
        return Err("Link di attivazione Twitch non valido".into());
    }
    Ok(())
}

fn open_browser(value: &str) -> Result<(), String> {
    #[cfg(windows)]
    {
        use windows_sys::Win32::UI::Shell::ShellExecuteW;
        let wide: Vec<u16> = value.encode_utf16().chain(Some(0)).collect();
        let verb: Vec<u16> = "open\0".encode_utf16().collect();
        let result = unsafe {
            ShellExecuteW(
                std::ptr::null_mut(),
                verb.as_ptr(),
                wide.as_ptr(),
                std::ptr::null(),
                std::ptr::null(),
                1,
            )
        };
        if result as isize <= 32 {
            return Err("Impossibile aprire il browser predefinito".into());
        }
        Ok(())
    }
    #[cfg(not(windows))]
    {
        let _ = value;
        Err("Apertura browser disponibile su Windows".into())
    }
}

#[tauri::command]
pub fn chat_open_activation(state: State<'_, SharedChat>) -> Result<(), String> {
    let value = state
        .lock()
        .map_err(|_| "Chat non disponibile")?
        .snapshot
        .verification_uri
        .clone()
        .ok_or("Richiedi prima un codice Twitch")?;
    activation_url(&value)?;
    open_browser(&value)
}

#[tauri::command]
pub fn chat_open_developer_console() -> Result<(), String> {
    open_browser("https://dev.twitch.tv/console/apps")
}

async fn refresh(http: &Client, auth: &mut Credentials) -> Result<(), SessionError> {
    let response = http
        .post("https://id.twitch.tv/oauth2/token")
        .form(&[
            ("client_id", auth.client_id.as_str()),
            ("refresh_token", auth.refresh.as_str()),
            ("grant_type", "refresh_token"),
        ])
        .send()
        .await
        .map_err(|_| {
            (
                false,
                "Impossibile rinnovare l'accesso Twitch. Controlla la connessione".into(),
            )
        })?;
    if !response.status().is_success() {
        return Err((
            !response.status().is_server_error() && response.status().as_u16() != 429,
            "Accesso Twitch non rinnovato. Ricollega l'account se il problema persiste".into(),
        ));
    }
    let tokens: Tokens = response.json().await.map_err(|_| {
        (
            true,
            "Risposta Twitch non valida. Ricollega l'account".into(),
        )
    })?;
    auth.access = tokens.access_token;
    auth.refresh = tokens.refresh_token;
    Ok(())
}

async fn prepare_credentials(
    http: &Client,
    credentials: &SharedCredentials,
) -> Result<Credentials, SessionError> {
    let http = http.clone();
    let credentials = credentials.clone();
    // A device refresh token is single-use. Finish rotating it even if the user
    // changes channels and aborts the socket task while this request is in flight.
    tauri::async_runtime::spawn(async move {
        let mut auth = credentials.lock().await;
        let identity = match validate(&http, &auth).await {
            Err((false, error)) => return Err((false, error)),
            Ok(identity) if identity["expires_in"].as_u64().unwrap_or(0) >= 3700 => identity,
            _ => {
                refresh(&http, &mut auth).await?;
                validate(&http, &auth).await?
            }
        };
        auth.can_write = identity["scopes"].as_array().is_some_and(|s| s.iter().any(|s| s == WRITE_SCOPE));
        Ok(auth.clone())
    })
    .await
    .map_err(|_| (true, "Rinnovo Twitch interrotto".to_string()))?
}

async fn run_chat(
    app: AppHandle,
    shared: SharedChat,
    epoch: u64,
    channel: String,
    credentials: SharedCredentials,
) {
    let http = client();
    let mut delay = 2;
    loop {
        let result = session(&app, &shared, epoch, &channel, &http, &credentials).await;
        match result {
            Err((true, error)) => {
                status(&app, &shared, epoch, "Chat non disponibile", &error);
                return;
            }
            Err((false, error)) => status(&app, &shared, epoch, "Riconnessione chat…", &error),
            Ok(()) => {}
        }
        tokio::time::sleep(Duration::from_secs(delay)).await;
        delay = (delay * 2).min(30);
    }
}

type SessionError = (bool, String); // true: user action required, no automatic retry
async fn session(
    app: &AppHandle,
    shared: &SharedChat,
    epoch: u64,
    channel: &str,
    http: &Client,
    credentials: &SharedCredentials,
) -> Result<(), SessionError> {
    let mut auth = prepare_credentials(http, credentials).await?;
    let response = http
        .get("https://api.twitch.tv/helix/users")
        .query(&[("login", channel)])
        .bearer_auth(&auth.access)
        .header("Client-Id", &auth.client_id)
        .send()
        .await
        .map_err(|_| (false, "Twitch non raggiungibile".into()))?;
    if !response.status().is_success() {
        return Err((
            true,
            "Impossibile cercare il canale. Ricollega Twitch".into(),
        ));
    }
    let value: Value = response
        .json()
        .await
        .map_err(|_| (false, "Risposta canale non valida".into()))?;
    let broadcaster = value["data"][0]["id"]
        .as_str()
        .ok_or((true, "Canale Twitch inesistente".into()))?
        .to_owned();
    {
        let mut c = shared.lock().map_err(|_| (true, "Chat non disponibile".to_string()))?;
        if c.chat_epoch != epoch { return Err((true, "Sessione chat cambiata".into())); }
        c.broadcaster_id = Some(broadcaster.clone());
        c.snapshot.can_send = auth.can_write;
    }
    let (mut socket, _) = tokio::time::timeout(Duration::from_secs(15), connect_async(WS))
        .await
        .map_err(|_| (false, "Timeout connessione chat".into()))?
        .map_err(|_| (false, "Connessione chat non riuscita".into()))?;
    let mut keepalive = Duration::from_secs(15);
    let mut validated_at = Instant::now();
    loop {
        let frame = tokio::time::timeout(keepalive + Duration::from_secs(2), socket.next())
            .await
            .map_err(|_| (false, "La chat non risponde".into()))?
            .ok_or((false, "Chat disconnessa".into()))?
            .map_err(|_| (false, "Connessione chat interrotta".into()))?;
        let text = match frame {
            Message::Text(t) => t,
            Message::Ping(data) => {
                socket
                    .send(Message::Pong(data))
                    .await
                    .map_err(|_| (false, "Chat disconnessa".into()))?;
                continue;
            }
            Message::Close(_) => return Err((false, "Chat disconnessa".into())),
            _ => continue,
        };
        let v: Value =
            serde_json::from_str(&text).map_err(|_| (false, "Evento Twitch non valido".into()))?;
        match v["metadata"]["message_type"].as_str().unwrap_or("") {
            "session_welcome" => {
                keepalive = Duration::from_secs(
                    v["payload"]["session"]["keepalive_timeout_seconds"]
                        .as_u64()
                        .unwrap_or(10)
                        .clamp(5, 600),
                );
                let id = v["payload"]["session"]["id"]
                    .as_str()
                    .ok_or((false, "Sessione Twitch non valida".into()))?;
                // Subscribe concurrently: Twitch allows only 10 seconds after welcome.
                let mut requests = futures_util::stream::FuturesUnordered::new();
                for kind in [
                    "channel.chat.message",
                    "channel.chat.message_delete",
                    "channel.chat.clear",
                    "channel.chat.clear_user_messages",
                ] {
                    requests.push(http.post("https://api.twitch.tv/helix/eventsub/subscriptions")
                        .bearer_auth(&auth.access).header("Client-Id", &auth.client_id)
                        .json(&json!({"type":kind,"version":"1","condition":{"broadcaster_user_id":broadcaster,"user_id":auth.user_id},"transport":{"method":"websocket","session_id":id}})).send());
                }
                while let Some(response) = requests.next().await {
                    let response = response
                        .map_err(|_| (false, "Iscrizione alla chat non riuscita".into()))?;
                    match response.status().as_u16() {
                        202 => {},
                        401 => return Err((true, "Accesso Twitch scaduto. Ricollega l'account".into())),
                        403 => return Err((true, "Twitch non consente l'accesso a questa chat: verifica ban, timeout o limiti dell'account".into())),
                        n => return Err((n < 500 && n != 429, format!("Twitch ha rifiutato la chat (HTTP {n})"))),
                    }
                }
                status(app, shared, epoch, "In diretta", "");
            }
            "session_reconnect" => {
                let value = v["payload"]["session"]["reconnect_url"]
                    .as_str()
                    .ok_or((false, "Riconnessione Twitch non valida".into()))?;
                let url = url::Url::parse(value)
                    .map_err(|_| (true, "Endpoint chat non valido".into()))?;
                if url.scheme() != "wss"
                    || url.host_str() != Some("eventsub.wss.twitch.tv")
                    || url.port().is_some()
                    || !url.username().is_empty()
                    || url.password().is_some()
                {
                    return Err((true, "Endpoint chat non consentito".into()));
                }
                // Keep the old connection alive until the migrated session welcomes us.
                let (mut next, _) =
                    tokio::time::timeout(Duration::from_secs(15), connect_async(value))
                        .await
                        .map_err(|_| (false, "Timeout riconnessione chat".into()))?
                        .map_err(|_| (false, "Riconnessione chat non riuscita".into()))?;
                let welcome = tokio::time::timeout(Duration::from_secs(10), next.next())
                    .await
                    .map_err(|_| (false, "Timeout sessione chat".into()))?
                    .ok_or((false, "Sessione chat chiusa".into()))?
                    .map_err(|_| (false, "Sessione chat interrotta".into()))?;
                let w: Value = serde_json::from_str(welcome.to_text().unwrap_or(""))
                    .map_err(|_| (false, "Sessione chat non valida".into()))?;
                if w["metadata"]["message_type"] != "session_welcome" {
                    return Err((false, "Sessione chat non confermata".into()));
                }
                keepalive = Duration::from_secs(
                    w["payload"]["session"]["keepalive_timeout_seconds"]
                        .as_u64()
                        .unwrap_or(10)
                        .clamp(5, 600),
                );
                let _ = socket.close(None).await;
                socket = next;
            }
            "revocation" => {
                return Err((
                    true,
                    "Accesso alla chat revocato da Twitch. Verifica l'account e ricollegalo".into(),
                ))
            }
            "notification" => apply_notification(app, shared, epoch, &v),
            _ => {}
        }
        if validated_at.elapsed() >= Duration::from_secs(3600) {
            auth = prepare_credentials(http, credentials).await?;
            validated_at = Instant::now();
        }
    }
}

fn apply_notification(app: &AppHandle, shared: &SharedChat, epoch: u64, v: &Value) {
    let Ok(mut c) = shared.lock() else {
        return;
    };
    if let Some(delta) = reduce_notification(&mut c, epoch, v) {
        let _ = app.emit("twitch-chat-event", delta);
    }
}

fn reduce_notification(c: &mut Chat, epoch: u64, v: &Value) -> Option<Value> {
    if c.chat_epoch != epoch {
        return None;
    }
    let id = v["metadata"]["message_id"].as_str()?;
    if c.seen.iter().any(|s| s == id) {
        return None;
    }
    c.seen.push_back(id.into());
    if c.seen.len() > 1000 {
        c.seen.pop_front();
    }
    let event = &v["payload"]["event"];
    let kind = v["payload"]["subscription"]["type"].as_str().unwrap_or("");
    let delta = match kind {
        "channel.chat.message" => {
            let message = parse_message(event)?;
            if c.snapshot.messages.iter().any(|m| m.id == message.id) {
                return None;
            }
            c.snapshot.messages.push_back(message.clone());
            if c.snapshot.messages.len() > LIMIT {
                c.snapshot.messages.pop_front();
            }
            json!({"kind":"message","message":message})
        }
        "channel.chat.message_delete" => {
            let id = event["message_id"].as_str().unwrap_or("");
            c.snapshot.messages.retain(|m| m.id != id);
            json!({"kind":"delete","target":id})
        }
        "channel.chat.clear_user_messages" => {
            let id = event["target_user_id"].as_str().unwrap_or("");
            c.snapshot.messages.retain(|m| m.user_id != id);
            json!({"kind":"clearUser","target":id})
        }
        "channel.chat.clear" => {
            c.snapshot.messages.clear();
            json!({"kind":"clear"})
        }
        _ => return None,
    };
    c.snapshot.version += 1;
    let mut delta = delta;
    delta["version"] = c.snapshot.version.into();
    Some(delta)
}

fn parse_message(v: &Value) -> Option<ChatMessage> {
    Some(ChatMessage {
        id: v["message_id"].as_str()?.into(),
        user_id: v["chatter_user_id"].as_str()?.into(),
        name: v["chatter_user_name"].as_str()?.chars().take(64).collect(),
        text: v["message"]["text"].as_str()?.chars().take(2000).collect(),
        color: v["color"].as_str().unwrap_or("").into(),
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    fn notification(id: &str, kind: &str, event: Value) -> Value {
        json!({"metadata":{"message_id":id},"payload":{"subscription":{"type":kind},"event":event}})
    }
    #[test]
    fn bounds_history_deduplicates_and_applies_moderation() {
        let mut c = Chat::default();
        for i in 0..205 {
            let v = notification(
                &format!("e{i}"),
                "channel.chat.message",
                json!({"message_id":i.to_string(),"chatter_user_id":if i%2==0 {"alice"} else {"bob"},"chatter_user_name":"Name","message":{"text":"hi"}}),
            );
            assert!(reduce_notification(&mut c, 0, &v).is_some());
            assert!(reduce_notification(&mut c, 0, &v).is_none());
        }
        assert_eq!(c.snapshot.messages.len(), LIMIT);
        assert_eq!(c.snapshot.messages.front().unwrap().id, "5");
        assert!(reduce_notification(
            &mut c,
            1,
            &notification("old", "channel.chat.clear", json!({}))
        )
        .is_none());
        assert_eq!(c.snapshot.messages.len(), LIMIT);
        reduce_notification(
            &mut c,
            0,
            &notification(
                "delete",
                "channel.chat.message_delete",
                json!({"message_id":"5"}),
            ),
        );
        assert!(!c.snapshot.messages.iter().any(|m| m.id == "5"));
        reduce_notification(
            &mut c,
            0,
            &notification(
                "timeout",
                "channel.chat.clear_user_messages",
                json!({"target_user_id":"alice"}),
            ),
        );
        assert!(c.snapshot.messages.iter().all(|m| m.user_id == "bob"));
        reduce_notification(
            &mut c,
            0,
            &notification("clear", "channel.chat.clear", json!({})),
        );
        assert!(c.snapshot.messages.is_empty());
        assert_eq!(c.snapshot.version, 208);
    }
    #[test]
    fn validates_external_destinations_and_channel_names() {
        assert!(activation_url("https://www.twitch.tv/activate?device-code=ABC").is_ok());
        for url in [
            "https://evil.test/activate",
            "https://www.twitch.tv@evil.test/activate",
            "http://www.twitch.tv/activate",
            "https://www.twitch.tv:444/activate",
        ] {
            assert!(activation_url(url).is_err());
        }
        assert!(valid_channel("twitch_123"));
        for name in ["", "foo/bar", "foo\r\nJOIN #other", "a b"] {
            assert!(!valid_channel(name));
        }
    }
    #[test]
    fn parses_untrusted_message_as_plain_text() {
        let message = parse_message(&json!({"message_id":"1","chatter_user_id":"2","chatter_user_name":"Alice","message":{"text":"<script>hello</script>"}})).unwrap();
        assert_eq!(message.text, "<script>hello</script>");
        assert!(parse_message(&json!({"message":{}})).is_none());
    }
}

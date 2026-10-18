# Twitch chat

Chat runs independently of MPV. It follows the playing Twitch channel, disconnects on stop or YouTube playback, and stays live while audio is paused. Chat failures never control playback or advance the queue.

## Setup

1. Open **Impostazioni → Account Twitch** and its developer console link.
2. Register an app in the [Twitch Developer Console](https://dev.twitch.tv/console/apps): **Public** client type, desktop application category, redirect `http://localhost`. Device-code login does not use that redirect. Twitch may require developer account verification.
3. Paste the **Client ID**, not a Client Secret. This public identifier is remembered in preferences; the OAuth session is stored separately in Windows Credential Manager.
4. Select **Collega Twitch**, then **Apri Twitch**. Check the code and authorize `user:read:chat` and `user:write:chat` in your browser. Your password stays on Twitch.
5. Play a Twitch channel: chat appears below the audio/video player. Type a message and press Send or Enter.

A separate bot account or moderator role is not required with a user token. Bans, timeouts, account limits and revocation can prevent access. Disconnecting in the app ends its local session; revoke consent in Twitch account connections if needed.

Existing read-only sessions need renewed consent: **Impostazioni → Account Twitch → Abilita invio messaggi**. Messages use your own account, have a 500-character maximum and require the current channel's chat to be connected. Rejections such as AutoMod, bans and Twitch limits keep your draft intact. Sends are never retried automatically; check chat before retrying an uncertain outcome. Published messages arrive through EventSub. In Shared Chat sessions, Twitch may distribute messages to other participating channels under its own rules.

Changing channel or account clears the draft. A send still being prepared is cancelled if its session changes; a message already transmitted to Twitch cannot be retracted by switching channels.

Video controls use owned transparent windows above the native MPV host. Mouse entry/movement reveals them; two seconds of inactivity hides them. Hovering the buttons keeps them visible. They take no space away from the picture, including for YouTube, fullscreen and detached video. They hide with the player or when another application takes foreground focus.

The chat header's window icon opens an independent view of the same account, channel and buffer. Only the main player selects channels; detaching creates no duplicate chat connection. Docking or closing that window restores inline chat. Detached state is not remembered across launches.

Live mode follows the latest message, including after resizing. Hovering messages freezes incoming rows for reading; leaving resumes live unless you scrolled back. Scrolling up holds the reading pause; reaching the bottom or selecting **Torna al live** resumes. Moderation still updates frozen rows. Each view holds at most 200 frozen rows in addition to the 200 live rows, not a complete history.

## Limitations

- Access/refresh tokens live in Rust and Windows Credential Manager, protected under the current Windows profile, never localStorage, JSON files or the repository. After logging in with this version, startup restores and validates the session and renews tokens before expiry. Disconnect deletes the stored credential and prevents an in-flight renewal from saving it again. Revocation, refresh-token expiry, vault errors or competing app instances can require login again: use one app instance at a time.
- Chat connects even while its view is closed when a channel and account are available. It keeps at most 200 messages in memory.
- New messages only, with no earlier history or recovery of messages missed during outages.
- Plain text, usernames and colors. Emotes stay text; badges, emote images, subscription notices and VOD replay are not included.
- Handles message deletion, user message clearing and channel clearing. Deduplication uses a bounded event buffer.
- Network errors reconnect with increasing delay; denied or revoked access requires user action. Changing channels recreates EventSub.
- Live chat may run ahead of buffered MPV/Streamlink playback; time synchronization is not implemented.

HTTPS and secure WebSockets run in Rust. Tokens never reach the frontend. Browser commands accept no arbitrary destination, only the fixed Twitch console or validated Twitch activation URLs.

## Windows verification

Previous read-only version checks on October 18, 2026: frontend build (`npm run build`), 10 Rust tests (`cargo test --offline --manifest-path src-tauri/Cargo.toml --lib`) and debug application compilation completed on Windows. PNG icon inspected. Native visual inspection was blocked by computer-use permissions; browser preview was unreachable. Real login/chat were not exercised without a Client ID and authorization.

Overlay, detached chat, scrolling and account persistence changes have not been built or tested, at the user's request. Verify hover/inactivity, resize/DPI, fullscreen, video detaching, Alt-Tab/tray; chat reading/moderation/live resumption/popout; full app exit/relaunch, token renewal, offline recovery and disconnect followed by restart.

Compilation and local tests do not replace real-account testing. A public Client ID and browser authorization are required to verify login, incoming messages, channel switching, stop, YouTube switching, network recovery, logout and moderation.

Also verify resizing, dragging from the top-left grip, tray, hiding embedded video in other views, detaching and fullscreen. The audio wave is decorative: it reflects recorded playback state and does not measure audio.

References: [Device Code Grant](https://dev.twitch.tv/docs/authentication/getting-tokens-oauth/#device-code-grant-flow), [receiving messages](https://dev.twitch.tv/docs/chat/send-receive-messages/#receiving-chat-messages), [EventSub WebSocket](https://dev.twitch.tv/docs/eventsub/handling-websocket-events/).

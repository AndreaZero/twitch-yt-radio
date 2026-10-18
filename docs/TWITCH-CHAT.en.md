# Twitch chat

Chat runs independently of MPV. It follows the playing Twitch channel, disconnects on stop or YouTube playback, and stays live while audio is paused. Chat failures never control playback or advance the queue.

## Setup

1. Open **Impostazioni → Account Twitch** and its developer console link.
2. Register an app in the [Twitch Developer Console](https://dev.twitch.tv/console/apps): **Public** client type, desktop application category, redirect `http://localhost`. Device-code login does not use that redirect. Twitch may require developer account verification.
3. Paste the **Client ID**, not a Client Secret. Only this public identifier is remembered locally.
4. Select **Collega Twitch**, then **Apri Twitch**. Check the code and authorize `user:read:chat` and `user:write:chat` in your browser. Your password stays on Twitch.
5. Play a Twitch channel: chat appears below the audio/video player. Type a message and press Send or Enter.

A separate bot account or moderator role is not required with a user token. Bans, timeouts, account limits and revocation can prevent access. Disconnecting in the app ends its local session; revoke consent in Twitch account connections if needed.

Existing read-only sessions need renewed consent: **Impostazioni → Account Twitch → Abilita invio messaggi**. Messages use your own account, have a 500-character maximum and require the current channel's chat to be connected. Rejections such as AutoMod, bans and Twitch limits keep your draft intact. Sends are never retried automatically; check chat before retrying an uncertain outcome. Published messages arrive through EventSub. In Shared Chat sessions, Twitch may distribute messages to other participating channels under its own rules.

Changing channel or account clears the draft. A send still being prepared is cancelled if its session changes; a message already transmitted to Twitch cannot be retracted by switching channels.

Controls share the video's frame in a reserved strip below the picture: the native MPV host remains separate from WebView controls and cannot cover the buttons. Fullscreen hides chat; normal playback shows it below the controls.

## Limitations

- Tokens and refresh tokens live only in Rust memory. Restarting requires login again; hiding to the tray preserves the session.
- Chat connects even while its view is closed when a channel and account are available. It keeps at most 200 messages in memory.
- New messages only, with no earlier history or recovery of messages missed during outages.
- Plain text, usernames and colors. Emotes stay text; badges, emote images, subscription notices and VOD replay are not included.
- Handles message deletion, user message clearing and channel clearing. Deduplication uses a bounded event buffer.
- Network errors reconnect with increasing delay; denied or revoked access requires user action. Changing channels recreates EventSub.
- Live chat may run ahead of buffered MPV/Streamlink playback; time synchronization is not implemented.

HTTPS and secure WebSockets run in Rust. Tokens never reach the frontend. Browser commands accept no arbitrary destination, only the fixed Twitch console or validated Twitch activation URLs.

## Windows verification

Previous read-only version checks on October 18, 2026: frontend build (`npm run build`), 10 Rust tests (`cargo test --offline --manifest-path src-tauri/Cargo.toml --lib`) and debug application compilation completed on Windows. PNG icon inspected. Native visual inspection was blocked by computer-use permissions; browser preview was unreachable. Real login/chat were not exercised without a Client ID and authorization.

The new inline chat, integrated controls and sending changes have not been built or tested, at the user's request. Verify successful/rejected sends, permission upgrades and channel switching during a send.

Compilation and local tests do not replace real-account testing. A public Client ID and browser authorization are required to verify login, incoming messages, channel switching, stop, YouTube switching, network recovery, logout and moderation.

Also verify resizing, dragging from the top-left grip, tray, hiding embedded video in other views, detaching and fullscreen. The audio wave is decorative: it reflects recorded playback state and does not measure audio.

References: [Device Code Grant](https://dev.twitch.tv/docs/authentication/getting-tokens-oauth/#device-code-grant-flow), [receiving messages](https://dev.twitch.tv/docs/chat/send-receive-messages/#receiving-chat-messages), [EventSub WebSocket](https://dev.twitch.tv/docs/eventsub/handling-websocket-events/).

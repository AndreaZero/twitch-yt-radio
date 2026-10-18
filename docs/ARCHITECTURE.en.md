# Architecture

[English](ARCHITECTURE.en.md) | [Italiano](ARCHITECTURE.it.md) | [Home](../README.md)

## Components

| File | Responsibility |
| --- | --- |
| `src/App.tsx` | React UI, URL entry, queue, local persistence, dependency diagnostics, playback events. |
| `src/TwitchChat.tsx` | Chat state and views, device-code login and account setup. |
| `src/YoutubeChat.tsx` / `src-tauri/src/youtube.rs` | YouTube live chat and video comments; desktop PKCE OAuth, Google renewal, API-paced polling, pagination and validated writes. Reuses the Twitch live list. |
| `src-tauri/src/chat.rs` | Twitch OAuth, in-memory tokens, EventSub, bounded history and moderation; validated Helix sending for the current channel/account and Twitch rejection handling. |
| `src/VideoPanel.tsx` | Video placeholder measurement, view controls, detached window UI and synchronized playback controls. |
| `src/VideoControls.tsx` / `src-tauri/src/video_overlay.rs` | Owned transparent control windows, native video hover/timeout and synchronized commands. |
| `src-tauri/src/token_store.rs` | Windows credentials, session restoration and protection against post-logout persistence races. |
| `src/styles.css` | Compact desktop layout. |
| `src-tauri/src/lib.rs` | Validated Rust commands, process lifecycle, Twitch resolution, MPV IPC, tray. |
| `src-tauri/src/playback.rs` | Validated playback modes and format/input options. |
| `src-tauri/src/video_host.rs` | Windows child host, validated/clipped physical bounds, fullscreen and reparenting between app windows. |
| `src-tauri/src/dependencies.rs` | Resolve and validate external executable paths, including Windows install locations. |
| `src-tauri/src/main.rs` | Native entry point. |
| `src-tauri/capabilities/default.json` | Tauri capability configuration; no frontend shell plugin. |
| `src-tauri/tauri.conf.json` | Window, content security policy, Vite integration, NSIS packaging. |

## Playback flow

1. React validates the URL and invokes `start_playback` with URL, volume and mode (`audio_only` or `video_audio`). Omitting mode defaults to audio; unknown modes are rejected by deserialization.
2. Rust validates HTTPS, rejects embedded credentials, limits URL length, and allows only the configured YouTube/Twitch hostnames.
3. Audio mode uses YouTube `bestaudio` or Streamlink `audio_only`, with no video fallback. Video mode uses YouTube `bestvideo[height<=720]+bestaudio/best[height<=720]` or Twitch `best`.
4. Rust launches MPV without a console. Audio mode disables video; video mode passes `--wid=<uint32 HWND>` for an app-owned native child host over the React placeholder. Native operations run on Tauri's UI thread. React sends only bounds and view intent, never a window handle or shell command. Default MPV input bindings, keyboard/cursor input and OSC are disabled so controls stay in React.
5. Pause and volume are sent as JSON to a local Windows named pipe. Stop terminates the child process.
6. A monitor polls MPV exit status and emits `playback-ended`. A generation counter rejects stale completion events after a track switch or stop.
7. React advances only after a successful exit. Failed exits display an error and do not automatically skip. Completion events arriving during a start are deferred until its result, then checked against the current generation. The native surface hides on completion only if the generation is still current.

The hidden `video` webview window is created with the main window. Detach/reinsert reparents the app-owned host between these two windows with Win32 `SetParent`; MPV remains its child and keeps playing. Fullscreen uses the owning Tauri window and a React layout that reserves a visible control strip. Closing the detached window is intercepted and docks the video. Bounds use device pixel ratio, client-area clipping and revisions to reject out-of-order layout updates. The inactive window cannot move/hide the active surface. A DPI/resize event triggers new measurements. Main settings hide an embedded surface while retaining playback. There is no browser media decoding or local media proxy.

Mode switches reopen the current URL through the same start command, reset pause, restart YouTube from the beginning and reconnect Twitch to live. Dependency/source resolution and process spawn errors leave the existing player intact. A later asynchronous MPV failure stops playback. There is no seamless switch or position restoration.

Pause, volume, start and stop emit recorded playback snapshots so controls in both webviews stay synchronized. There is no two-way MPV IPC state confirmation: pause and volume still reflect locally recorded command intent. The app does not currently report buffering or audible output.

## State and lifecycle

- Rust owns the MPV child process inside `Arc<Mutex<Player>>`.
- Queue, volume and preferred mode live in WebView2 `localStorage`; no cloud synchronization. An active backend snapshot takes precedence after a UI reload.
- Closing the main window hides it and any embedded video. A detached video remains visible; closing it reinserts it and shows the widget. Stop or audio mode closes the popout and exits fullscreen. Normal queue advancement preserves video placement. Tray Quit kills MPV and exits.
- Video placement defaults to embedded on every app launch. The selected audio/video mode remains persisted; no existing audio preference is silently changed to video.
- Existing internal identifiers (`background-audio`, `background_audio_lib`, and `dev.andreazero.backgroundaudio`) are retained. The public repository name is `twitch-yt-radio`.
- Packaging does not include external playback binaries or publish releases.

## Development constraints

Keep React UI and Rust/MPV engine responsibilities separate. Never expose a shell in the frontend or silently replace Twitch `audio_only` with video. Prefer small isolated changes and concrete errors. Do not advance automatically after playback errors. Avoid unnecessary dependencies and repeated build/test loops. Document actual assumptions and Windows checks performed; do not claim unexecuted behavior works.

The original local development instructions remain in [AGENTS.md](../AGENTS.md). See [CONTRIBUTING.md](../CONTRIBUTING.md) for the bilingual contribution process.

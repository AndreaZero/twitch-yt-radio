# Architecture

[English](ARCHITECTURE.en.md) | [Italiano](ARCHITECTURE.it.md) | [Home](../README.md)

## Components

| File | Responsibility |
| --- | --- |
| `src/App.tsx` | React UI, URL entry, queue, local persistence, dependency diagnostics, playback events. |
| `src/styles.css` | Compact desktop layout. |
| `src-tauri/src/lib.rs` | Validated Rust commands, process lifecycle, Twitch resolution, MPV IPC, tray. |
| `src-tauri/src/playback.rs` | Validated playback modes, format/window options and manual video close classification. |
| `src-tauri/src/dependencies.rs` | Resolve and validate external executable paths, including Windows install locations. |
| `src-tauri/src/main.rs` | Native entry point. |
| `src-tauri/capabilities/default.json` | Tauri capability configuration; no frontend shell plugin. |
| `src-tauri/tauri.conf.json` | Window, content security policy, Vite integration, NSIS packaging. |

## Playback flow

1. React validates the URL and invokes `start_playback` with URL, volume and mode (`audio_only` or `video_audio`). Omitting mode defaults to audio; unknown modes are rejected by deserialization.
2. Rust validates HTTPS, rejects embedded credentials, limits URL length, and allows only the configured YouTube/Twitch hostnames.
3. Audio mode uses YouTube `bestaudio` or Streamlink `audio_only`, with no video fallback. Video mode uses YouTube `bestvideo[height<=720]+bestaudio/best[height<=720]` or Twitch `best`.
4. Rust launches MPV without a console, hiding video in audio mode or opening a separate always-on-top mini window in video mode (`--autofit=384x216`). External executables are invoked directly using arguments, not shell strings. A fixed input configuration in the app cache maps `CLOSE_WIN` to `quit 10`; other default input bindings and OSC are disabled so controls stay in React.
5. Pause and volume are sent as JSON to a local Windows named pipe. Stop terminates the child process.
6. A monitor polls MPV exit status and emits `playback-ended`. A generation counter rejects stale completion events after a track switch or stop.
7. React advances only after a successful exit. Failed exits display an error and do not automatically skip. Video exit code 10 means manual close, stopping without error or advancement. Completion events arriving during a start are deferred until its result, then checked against the current generation.

Mode switches reopen the current URL through the same start command, reset pause, restart YouTube from the beginning and reconnect Twitch to live. Dependency/source resolution and process spawn errors leave the existing player intact. A later asynchronous MPV failure stops playback. There is no seamless switch or position restoration.

There is no two-way IPC state confirmation: pause and volume reflect locally recorded command intent. The app does not currently report buffering or audible output.

## State and lifecycle

- Rust owns the MPV child process inside `Arc<Mutex<Player>>`.
- Queue, volume and preferred mode live in WebView2 `localStorage`; no cloud synchronization. An active backend snapshot takes precedence after a UI reload.
- Closing the main window hides it. The separate video window remains visible until closed, stopped or switched to audio mode. Tray Quit kills MPV and exits.
- Existing internal identifiers (`background-audio`, `background_audio_lib`, and `dev.andreazero.backgroundaudio`) are retained. The public repository name is `twitch-yt-radio`.
- Packaging does not include external playback binaries or publish releases.

## Development constraints

Keep React UI and Rust/MPV engine responsibilities separate. Never expose a shell in the frontend or silently replace Twitch `audio_only` with video. Prefer small isolated changes and concrete errors. Do not advance automatically after playback errors. Avoid unnecessary dependencies and repeated build/test loops. Document actual assumptions and Windows checks performed; do not claim unexecuted behavior works.

The original local development instructions remain in [AGENTS.md](../AGENTS.md). See [CONTRIBUTING.md](../CONTRIBUTING.md) for the bilingual contribution process.

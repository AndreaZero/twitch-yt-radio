# Architecture

[English](ARCHITECTURE.en.md) | [Italiano](ARCHITECTURE.it.md) | [Home](../README.md)

## Components

| File | Responsibility |
| --- | --- |
| `src/App.tsx` | React UI, URL entry, queue, local persistence, dependency diagnostics, playback events. |
| `src/styles.css` | Compact desktop layout. |
| `src-tauri/src/lib.rs` | Validated Rust commands, process lifecycle, Twitch resolution, MPV IPC, tray. |
| `src-tauri/src/main.rs` | Native entry point. |
| `src-tauri/capabilities/default.json` | Tauri capability configuration; no frontend shell plugin. |
| `src-tauri/tauri.conf.json` | Window, content security policy, Vite integration, NSIS packaging. |

## Playback flow

1. React validates the URL and invokes `start_playback` with URL and volume.
2. Rust validates HTTPS, rejects embedded credentials, limits URL length, and allows only the configured YouTube/Twitch hostnames.
3. YouTube is passed to MPV with `--ytdl-format=bestaudio`; Twitch is resolved with `streamlink --stream-url URL audio_only`.
4. Rust launches MPV without a video window or console. External executables are invoked directly using arguments, not shell strings.
5. Pause and volume are sent as JSON to a local Windows named pipe. Stop terminates the child process.
6. A monitor polls MPV exit status and emits `playback-ended`. A generation counter rejects stale completion events after a track switch or stop.
7. React advances only after a successful exit. Failed exits display an error and do not automatically skip.

There is no two-way IPC state confirmation: pause and volume reflect locally recorded command intent. The app does not currently report buffering or audible output.

## State and lifecycle

- Rust owns the MPV child process inside `Arc<Mutex<Player>>`.
- Queue and volume live in WebView2 `localStorage`; no cloud synchronization.
- Closing the main window hides it. Tray Quit kills MPV and exits.
- Existing internal identifiers (`background-audio`, `background_audio_lib`, and `dev.andreazero.backgroundaudio`) are retained. The public repository name is `twitch-yt-radio`.
- Packaging does not include external playback binaries or publish releases.

## Development constraints

Keep React UI and Rust/MPV engine responsibilities separate. Never expose a shell in the frontend or silently replace Twitch `audio_only` with video. Prefer small isolated changes and concrete errors. Do not advance automatically after playback errors. Avoid unnecessary dependencies and repeated build/test loops. Document actual assumptions and Windows checks performed; do not claim unexecuted behavior works.

The original local development instructions remain in [AGENTS.md](../AGENTS.md). See [CONTRIBUTING.md](../CONTRIBUTING.md) for the bilingual contribution process.

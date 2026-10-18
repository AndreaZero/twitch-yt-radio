# twitch-yt-radio

[English](README.md) | [Italiano](README.it.md)

A compact Windows 11 desktop player for **YouTube and Twitch, with audio-only or optional video playback**, built with Tauri 2, React, TypeScript, Rust, and MPV.

**Status:** personal MVP, version 0.1.0. Source code only: there are no published releases or downloadable installers. Build a local Windows installer using the [installation guide](docs/INSTALL.en.md).

The current application is displayed as **Background Audio**. The interface and playback errors are currently in Italian; documentation is available in English and Italian.

## Features

- Add individual YouTube videos or live Twitch channels to a local queue.
- Choose **Solo audio** (default) or **Video + audio** in the widget. The choice is saved locally.
- In audio mode, YouTube requests `bestaudio`; Twitch requests Streamlink's `audio_only` and reports an error when unavailable, without video fallback.
- Video mode opens a separate, resizable MPV mini window, initially fitted within 384 × 216 pixels (video area), always on top. The control widget keeps its size; pause, volume and stop stay there.
- Video mode requests YouTube video up to 720p plus audio, or Twitch's `best` stream. Twitch video may use substantially more bandwidth.
- Changing mode during playback reopens the source: YouTube starts over and Twitch reconnects to live. Playback resumes unpaused. This is not a seamless visibility toggle.
- Play, pause, stop, previous/next, mute, and volume controls.
- Advance after a normal MPV process exit; stop automatic advancement on a playback error or when closing the video window. To keep listening without video, choose **Solo audio** instead of closing MPV.
- Persist queue entries and volume in local WebView2 storage.
- Close the window to keep playing in the Windows tray; **Esci** (Quit) terminates the app and its MPV process.
- Check external dependencies in settings.

## Get started

Install the [Windows prerequisites and playback dependencies](docs/INSTALL.en.md), then execute each command separately in PowerShell:

```powershell
git clone https://github.com/AndreaZero/twitch-yt-radio.git
```

```powershell
cd twitch-yt-radio
```

```powershell
npm ci
```

```powershell
npm run tauri dev
```

To build a local NSIS installer:

```powershell
npm run tauri build
```

Installer output: `src-tauri\target\release\bundle\nsis\`. **MPV, yt-dlp, and Streamlink are not bundled** and must remain available in the app's PATH.

## Documentation

| Topic | English | Italiano |
| --- | --- | --- |
| Installation, development, local installer | [Guide](docs/INSTALL.en.md) | [Guida](docs/INSTALL.it.md) |
| PATH, playback, and build troubleshooting | [Guide](docs/TROUBLESHOOTING.en.md) | [Guida](docs/TROUBLESHOOTING.it.md) |
| Architecture and development rules | [Guide](docs/ARCHITECTURE.en.md) | [Guida](docs/ARCHITECTURE.it.md) |
| Contributions and Windows checks | [Contributing](CONTRIBUTING.md#english) | [Contribuire](CONTRIBUTING.md#italiano) |
| Security and privacy | [Policy](SECURITY.md#english) | [Informazioni](SECURITY.md#italiano) |

## Known limits

- Windows 11 is the target; controls use Windows named pipes. macOS and Linux are not supported by this MVP.
- Twitch audio mode requires a live channel that exposes `audio_only`; video mode explicitly requests a video stream instead.
- YouTube login requirements, anti-bot checks, DRM, and regional restrictions can prevent playback. External tools may need updating when platforms change.
- Entries are individual links; YouTube playlist import, full titles, and artwork are not implemented.
- No global media keys, live reconnection, retries, or automatic dependency updates.
- A running MPV process does not prove audio is audible or buffering has completed. MPV output is currently suppressed.
- No media export or download feature is provided.

The setup session confirmed executable discovery on the user's Windows machine. Repository preparation does not certify end-to-end streaming, controls, or a fresh installer build. See the [manual Windows checklist](CONTRIBUTING.md#english).

The optional video window and mode switching still require manual Windows playback checks; the app is not started as part of source verification. MPV's window sizing and close binding follow the [official MPV manual](https://mpv.io/manual/stable/).

Independent of Twitch and YouTube. Use in accordance with service terms and content rights. External tools retain their own licenses; their binaries are not redistributed here.

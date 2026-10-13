# Install and build on Windows

[English](INSTALL.en.md) | [Italiano](INSTALL.it.md) | [Home](../README.md)

No published releases are available. Build the source locally and run the generated installer. Commands target PowerShell on Windows 11: paste and execute **one command at a time**.

## Development prerequisites

- [Git for Windows](https://git-scm.com/downloads/win).
- [Node.js](https://nodejs.org/en/download) with npm: use a supported LTS version (Node 22 or 24).
- [Rust via rustup](https://rustup.rs/): stable, MSVC toolchain matching your Windows architecture.
- [Visual Studio Build Tools](https://visualstudio.microsoft.com/visual-cpp-build-tools/): **Desktop development with C++**, including MSVC and a Windows SDK.
- [Microsoft Edge WebView2 Runtime](https://developer.microsoft.com/en-us/microsoft-edge/webview2/), usually already installed on Windows 11.

Follow the [official Tauri Windows prerequisites](https://v2.tauri.app/start/prerequisites/#windows) for full toolchain setup. Build tools are separate from the playback tools below.

## Playback dependencies

Using Windows Package Manager, run separately:

```powershell
winget install --id shinchiro.mpv --exact --source winget
```

```powershell
winget install --id yt-dlp.yt-dlp --exact --source winget
```

```powershell
winget install streamlink --source winget
```

Enable **Add to PATH** if offered. WinGet reporting an installed package does not prove it is reachable in the current PATH.

Alternative setup: [MPV](https://mpv.io/installation/), [yt-dlp](https://github.com/yt-dlp/yt-dlp#installation), [Streamlink](https://streamlink.github.io/install.html). Check their current supporting requirements if extraction fails.

## Verify PATH

Open a new terminal. If it inherited an older PATH, refresh this session from Windows' saved values:

```powershell
$env:Path = [Environment]::GetEnvironmentVariable('Path','Machine') + ';' + [Environment]::GetEnvironmentVariable('Path','User')
```

```powershell
Get-Command mpv, yt-dlp, streamlink
```

```powershell
mpv --version
```

```powershell
yt-dlp --version
```

```powershell
streamlink --version
```

The app requires each `--version` probe to exit successfully. `Get-Command` only checks command discovery. For missing paths, see [troubleshooting](TROUBLESHOOTING.en.md#executables-not-found).

## Get the source

```powershell
git clone https://github.com/AndreaZero/twitch-yt-radio.git
```

```powershell
cd twitch-yt-radio
```

```powershell
npm ci
```

npm and Cargo lockfiles are included. The first Rust build downloads dependencies and may take several minutes. Dependencies and streaming require Internet access.

## Run in development

```powershell
npm run tauri dev
```

Starts Vite on port 1420 and the native app. `npm run dev` alone starts only the frontend server; native playback commands need Tauri.

Paste a YouTube video or live Twitch channel URL and press the add button. Settings contain diagnostics. In the Italian UI, **Aggiorna** means Refresh, **Trovato** means Found, and **Non rilevato** means Not detected. Closing the window keeps the player running; use tray menu **Esci** to quit.

## Build and install locally

From the repository root:

```powershell
npm run tauri build
```

Open the installer `.exe` in `src-tauri\target\release\bundle\nsis\` using File Explorer. The current product name is **Background Audio**; the standalone executable is `src-tauri\target\release\background-audio.exe`.

MPV, yt-dlp, and Streamlink are **not included**. No code signing or auto-update service is configured; Windows may warn about an unsigned local installer. These commands do not publish a GitHub release.

If the app still inherits an old PATH, quit it from the tray, refresh PATH above, then launch from the same PowerShell session at the repository root:

```powershell
& '.\src-tauri\target\release\background-audio.exe'
```

Verify the new installer and real playback on Windows using the [manual checklist](../CONTRIBUTING.md#english).

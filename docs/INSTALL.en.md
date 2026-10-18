# Install and build on Windows

[English](INSTALL.en.md) | [Italiano](INSTALL.it.md) | [Home](../README.md)

No published releases are available. Build the source locally and run the generated installer. Commands target PowerShell on Windows 11: paste and execute **one command at a time**.

## Run the compiled app: installer or standalone EXE

Running the compiled app does not require Git, Node, Python, Rust or Visual Studio. Those are development prerequisites only.

- **NSIS installer:** installs for the current user. If WebView2 is missing, setup downloads and opens Microsoft's bootstrapper. Launch the app afterward; missing media tools open guided setup in settings.
- **Standalone EXE:** runs directly. A missing WebView2 runtime triggers a native prompt to download from Microsoft and run its installer after Windows signature verification. Failure opens the official download page for recovery. Once WebView2 is available, the same in-app wizard runs.
- **Installa mancanti:** installs MPV (`shinchiro.mpv`), yt-dlp (`yt-dlp.yt-dlp`), Streamlink (`Streamlink.Streamlink`), FFmpeg/FFprobe (`Gyan.FFmpeg.Essentials`) and Deno (`DenoLand.Deno`) in sequence, skipping usable tools. These are WinGet packages, not embedded binaries. The button accepts package/source terms; Windows may request elevation. Hash/security checks remain enabled.
- If WinGet is missing, open Microsoft's **App Installer** from the wizard, install/update it, then select **Rileva di nuovo**. If Store/policies prevent this, each tool provides its official website and a field for a locally installed/extracted executable. FFmpeg requires FFprobe in the same directory.

No environment variables need manual copying: the app reads inherited and saved Windows PATH values, discovers installation directories and supplies media child processes with their own PATH. Optional local overrides are saved in `tools.json` in the app configuration directory. A `bin` directory next to the EXE is supported; keep tool DLLs/support files too. Successful setup does not require an app restart. Third-party installers may update Windows PATH according to their own behavior.

Downloading tools requires Internet. The EXE is not an offline self-contained package: settings, accounts and tools remain on the profile/PC when moving it. Keep the app running until installation completes. There is no forced installer cancellation; inspect Windows processes after a timeout before retrying. Automatic tool updates are not implemented. Optional Twitch/Google accounts use their settings forms, without token/client-secret environment variables.

Sources: [Tauri installer](https://tauri.app/distribute/windows-installer/), [Microsoft WebView2 deployment](https://learn.microsoft.com/en-us/microsoft-edge/webview2/concepts/distribution), [WinGet install](https://learn.microsoft.com/en-us/windows/package-manager/winget/install), [yt-dlp requirements](https://github.com/yt-dlp/yt-dlp#dependencies).

## Resize video and chat

Drag the divider between video and chat/comments, including with touch. Arrow Up/Down adjusts it; Home/End reaches its limits. Double-click or Enter restores automatic sizing. The chosen proportion is saved and clamped to available space so both panels remain accessible. Detached/fullscreen video uses its own space without this divider.

The previous 320 px video cap is removed. Automatic height grows with the window and uses 16:9 as a layout hint; MPV preserves the source's actual aspect ratio. A wider frame may still have side bars: move the divider down to enlarge the image without stretching or cropping it.

This is development only: **no builds, tests, trial installations or app launches were performed**. Pending user Windows checks: missing tools/WinGet/WebView2, cancellation/offline recovery, UAC, paths with spaces, restart persistence, Deno/FFmpeg playback, dragging across native MPV, resizing/DPI, chat popout and fullscreen.

## Development prerequisites

- [Git for Windows](https://git-scm.com/downloads/win).
- [Node.js](https://nodejs.org/en/download) with npm: use a supported LTS version (Node 22 or 24).
- [Rust via rustup](https://rustup.rs/): stable, MSVC toolchain matching your Windows architecture.
- [Visual Studio Build Tools](https://visualstudio.microsoft.com/visual-cpp-build-tools/): **Desktop development with C++**, including MSVC and a Windows SDK.
- [Microsoft Edge WebView2 Runtime](https://developer.microsoft.com/en-us/microsoft-edge/webview2/), usually already installed on Windows 11.

Follow the [official Tauri Windows prerequisites](https://v2.tauri.app/start/prerequisites/#windows) for full toolchain setup. Build tools are separate from the playback tools below.

## Manual tool installation alternative

Using Windows Package Manager, run separately:

```powershell
winget install --id shinchiro.mpv --exact --source winget
```

```powershell
winget install --id yt-dlp.yt-dlp --exact --source winget
```

```powershell
winget install --id Streamlink.Streamlink --exact --source winget
```

```powershell
winget install --id Gyan.FFmpeg.Essentials --exact --source winget
```

```powershell
winget install --id DenoLand.Deno --exact --source winget
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

Media tools are **not embedded**: use the first-launch wizard described above. No app code signing or auto-update service is configured; Windows may warn about an unsigned local installer. These commands do not publish a GitHub release.

If the app still inherits an old PATH, quit it from the tray, refresh PATH above, then launch from the same PowerShell session at the repository root:

```powershell
& '.\src-tauri\target\release\background-audio.exe'
```

Verify the new installer and real playback on Windows using the [manual checklist](../CONTRIBUTING.md#english).

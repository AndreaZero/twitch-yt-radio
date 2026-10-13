# Troubleshooting

[English](TROUBLESHOOTING.en.md) | [Italiano](TROUBLESHOOTING.it.md) | [Home](../README.md)

Run PowerShell commands one at a time. Never overwrite the entire saved PATH with only the three playback folders.

## Executables not found

An installed WinGet package and an executable reachable through PATH are different things. Start with:

```powershell
$env:Path = [Environment]::GetEnvironmentVariable('Path','Machine') + ';' + [Environment]::GetEnvironmentVariable('Path','User')
```

```powershell
Get-Command mpv, yt-dlp, streamlink
```

Typical folders (installation method and user can change them):

| Tool | Typical folder |
| --- | --- |
| MPV | `C:\Program Files\MPV Player` |
| yt-dlp installed through WinGet | `%LOCALAPPDATA%\Microsoft\WinGet\Links` |
| Streamlink user installation | `%LOCALAPPDATA%\Programs\Streamlink\bin` |

Check that the expected `.exe` actually exists. If it does, use Windows **Edit environment variables for your account → Path → Edit → New**, and add its containing folder, not the executable path. Preserve existing entries. If it does not exist, install the tool using the [setup guide](INSTALL.en.md).

For MPV installed at exactly the typical location, this single line validates the file and appends its folder to the user PATH without discarding existing entries:

```powershell
if (Test-Path -LiteralPath 'C:\Program Files\MPV Player\mpv.exe') { $p = [Environment]::GetEnvironmentVariable('Path','User'); $p = (($p -split ';') + 'C:\Program Files\MPV Player' | Where-Object { $_ } | Select-Object -Unique) -join ';'; [Environment]::SetEnvironmentVariable('Path',$p,'User') } else { Write-Host 'mpv.exe not found at this location' }
```

Refresh the session PATH using the first command again, then test all three `--version` commands in the installation guide. `mpv.com` in PowerShell's output can be normal; verify `mpv.exe` is also installed because Rust launches MPV directly.

## Terminal finds tools, app does not

Processes inherit PATH from their parent at startup. Closing only the player window leaves it alive in the tray. Choose **Esci** in the tray menu, refresh PowerShell's PATH, and launch the built app from that terminal at the repository root:

```powershell
& '.\src-tauri\target\release\background-audio.exe'
```

For development use `npm run tauri dev` in that terminal instead. A full Windows sign-out/sign-in also refreshes the desktop environment. In settings, **Aggiorna** repeats dependency checks but does not reload the running process environment.

## Twitch does not start

The channel must be live and expose `audio_only`. Diagnose separately from the app:

```powershell
streamlink https://www.twitch.tv/CHANNEL
```

Replace `CHANNEL` with the actual channel name. The available-stream list must include `audio_only`. Offline channels, authentication requirements, service changes, and missing audio streams can prevent playback. Do not switch to `best` as a hidden workaround: this project deliberately requires audio-only Twitch streams.

## YouTube does not start

Check yt-dlp can see an audio format:

```powershell
yt-dlp --list-formats 'https://www.youtube.com/watch?v=VIDEO_ID'
```

Replace `VIDEO_ID` with a real video. This lists formats without downloading media. Review yt-dlp's current requirements and errors; login, anti-bot checks, DRM, and region restrictions may apply. The app does not currently configure cookies or a login flow.

## Updating playback tools

Use the same installation method originally used. For WinGet installs:

```powershell
winget upgrade --id shinchiro.mpv --exact --source winget
```

```powershell
winget upgrade --id yt-dlp.yt-dlp --exact --source winget
```

```powershell
winget upgrade streamlink --source winget
```

## Build problems

| Symptom | Action |
| --- | --- |
| Missing `link.exe` or Windows libraries | Install the C++ desktop workload and Windows SDK from the Tauri prerequisites. |
| Missing WebView2 | Install the WebView2 Runtime. |
| Port 1420 already in use | Close the previous development server or app session before starting another. |
| `EndEvent: Clone` compile error | Use current source: `EndEvent` derives both `Serialize` and `Clone` for Tauri's `emit`. |
| `npm ci` dependency error | Use supported Node LTS and the committed `package-lock.json`; include the actual error in a report. |

No automated test suite currently verifies streaming or tray behavior. Report the exact failing command, relevant error text, Windows version, and tool versions. Remove private URLs and tokens before sharing logs.

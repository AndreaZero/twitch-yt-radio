# Installazione e build su Windows

[English](INSTALL.en.md) | [Italiano](INSTALL.it.md) | [Home](../README.it.md)

Non ci sono release pubblicate. Compila il sorgente localmente e avvia l'installer generato. I comandi sono per PowerShell su Windows 11: incolla ed esegui **un comando alla volta**.

## Prerequisiti di sviluppo

- [Git per Windows](https://git-scm.com/downloads/win).
- [Node.js](https://nodejs.org/en/download) con npm: usa una versione LTS supportata (Node 22 o 24).
- [Rust tramite rustup](https://rustup.rs/): stable, toolchain MSVC adatta all'architettura Windows.
- [Visual Studio Build Tools](https://visualstudio.microsoft.com/visual-cpp-build-tools/): **Sviluppo di applicazioni desktop con C++**, includendo MSVC e un Windows SDK.
- [Microsoft Edge WebView2 Runtime](https://developer.microsoft.com/en-us/microsoft-edge/webview2/), normalmente già presente su Windows 11.

Segui i [prerequisiti ufficiali Tauri per Windows](https://v2.tauri.app/start/prerequisites/#windows). Gli strumenti di compilazione sono separati dalle dipendenze di riproduzione seguenti.

## Dipendenze audio

Con Windows Package Manager, esegui separatamente:

```powershell
winget install --id shinchiro.mpv --exact --source winget
```

```powershell
winget install --id yt-dlp.yt-dlp --exact --source winget
```

```powershell
winget install streamlink --source winget
```

Abilita **Add to PATH** se proposto. Un pacchetto già installato secondo WinGet non è necessariamente raggiungibile nel PATH attuale.

Configurazione alternativa: [MPV](https://mpv.io/installation/), [yt-dlp](https://github.com/yt-dlp/yt-dlp#installation), [Streamlink](https://streamlink.github.io/install.html). Verifica i loro requisiti aggiuntivi aggiornati se l'estrazione fallisce.

## Verifica del PATH

Apri un nuovo terminale. Se eredita ancora il vecchio PATH, aggiorna la sessione dai valori salvati in Windows:

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

L'app richiede che ogni verifica `--version` termini senza errori. `Get-Command` verifica soltanto il rilevamento del comando. Per percorsi mancanti, vedi [risoluzione problemi](TROUBLESHOOTING.it.md#eseguibili-non-trovati).

## Scarica il sorgente

```powershell
git clone https://github.com/AndreaZero/twitch-yt-radio.git
```

```powershell
cd twitch-yt-radio
```

```powershell
npm ci
```

I lockfile npm e Cargo sono inclusi. La prima compilazione Rust scarica dipendenze e può richiedere diversi minuti. Dipendenze e streaming richiedono accesso Internet.

## Avvio in sviluppo

```powershell
npm run tauri dev
```

Avvia Vite sulla porta 1420 e l'app nativa. `npm run dev` da solo avvia soltanto il server frontend; i comandi di riproduzione richiedono Tauri.

Incolla un link a un video YouTube o un canale Twitch live e premi il pulsante di aggiunta. Le impostazioni contengono la diagnostica. Chiudendo la finestra il player resta attivo; dal menu tray scegli **Esci** per terminare.

## Build e installazione locale

Dalla cartella principale della repository:

```powershell
npm run tauri build
```

Apri da Esplora file l'installer `.exe` in `src-tauri\target\release\bundle\nsis\`. Il prodotto attuale si chiama **Background Audio**; l'eseguibile della build è `src-tauri\target\release\background-audio.exe`.

MPV, yt-dlp e Streamlink **non sono inclusi**. Firma del codice e aggiornamento automatico non sono configurati; Windows può mostrare un avviso per un installer locale non firmato. Questi comandi non pubblicano una release GitHub.

Se l'app eredita ancora il vecchio PATH, esci dalla tray, aggiorna il PATH come sopra e avviala dalla stessa sessione PowerShell nella cartella principale:

```powershell
& '.\src-tauri\target\release\background-audio.exe'
```

Verifica il nuovo installer e la riproduzione reale su Windows con la [checklist manuale](../CONTRIBUTING.md#italiano).

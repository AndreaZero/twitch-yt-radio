# Risoluzione problemi

[English](TROUBLESHOOTING.en.md) | [Italiano](TROUBLESHOOTING.it.md) | [Home](../README.it.md)

Esegui i comandi PowerShell uno alla volta. Non sostituire tutto il PATH salvato con le sole tre cartelle delle dipendenze audio.

## Eseguibili non trovati

Un pacchetto installato secondo WinGet e un eseguibile raggiungibile nel PATH sono due cose diverse. Inizia con:

```powershell
$env:Path = [Environment]::GetEnvironmentVariable('Path','Machine') + ';' + [Environment]::GetEnvironmentVariable('Path','User')
```

```powershell
Get-Command mpv, yt-dlp, streamlink
```

Cartelle tipiche (possono cambiare secondo utente e metodo di installazione):

| Strumento | Cartella tipica |
| --- | --- |
| MPV | `C:\Program Files\MPV Player` |
| yt-dlp tramite WinGet | `%LOCALAPPDATA%\Microsoft\WinGet\Links` |
| Streamlink per utente | `%LOCALAPPDATA%\Programs\Streamlink\bin` |

Verifica che l'`.exe` previsto esista davvero. Se esiste, usa **Modifica le variabili di ambiente per l'account → Path → Modifica → Nuovo** e aggiungi la cartella contenente, non il percorso dell'eseguibile. Conserva le voci esistenti. Se non esiste, installa lo strumento con la [guida](INSTALL.it.md).

Per MPV installato esattamente nel percorso tipico, questa singola riga verifica il file e aggiunge la cartella al PATH utente senza scartare le voci esistenti:

```powershell
if (Test-Path -LiteralPath 'C:\Program Files\MPV Player\mpv.exe') { $p = [Environment]::GetEnvironmentVariable('Path','User'); $p = (($p -split ';') + 'C:\Program Files\MPV Player' | Where-Object { $_ } | Select-Object -Unique) -join ';'; [Environment]::SetEnvironmentVariable('Path',$p,'User') } else { Write-Host 'mpv.exe non trovato in questa cartella' }
```

Ricarica il PATH della sessione con il primo comando, poi prova i tre comandi `--version` nella guida di installazione. `mpv.com` nell'output di PowerShell può essere normale; verifica che sia installato anche `mpv.exe`, perché Rust avvia MPV direttamente.

## Il terminale trova gli strumenti, l'app no

I processi ereditano il PATH dal processo che li avvia. Chiudere solo la finestra lascia il player attivo nella tray. Scegli **Esci** dal menu tray, aggiorna il PATH di PowerShell e avvia l'app compilata da quel terminale nella cartella principale della repository:

```powershell
& '.\src-tauri\target\release\background-audio.exe'
```

Per lo sviluppo usa invece `npm run tauri dev` nello stesso terminale. Anche disconnettersi e riconnettersi a Windows aggiorna l'ambiente desktop. **Aggiorna** nelle impostazioni ripete la diagnostica ma non ricarica l'ambiente del processo attivo.

## Twitch non parte

Il canale deve essere live ed esporre `audio_only`. Per diagnosticare fuori dall'app:

```powershell
streamlink https://www.twitch.tv/CHANNEL
```

Sostituisci `CHANNEL` con il nome reale. La lista dei flussi disponibili deve includere `audio_only`. Canali offline, autenticazione, cambiamenti del servizio e flussi audio mancanti possono impedire la riproduzione. Non passare a `best` come soluzione nascosta: il progetto richiede intenzionalmente Twitch solo audio.

## YouTube non parte

Verifica che yt-dlp trovi un formato audio:

```powershell
yt-dlp --list-formats 'https://www.youtube.com/watch?v=VIDEO_ID'
```

Sostituisci `VIDEO_ID` con un video reale. Il comando elenca i formati senza scaricare media. Consulta requisiti aggiornati ed errori di yt-dlp; possono intervenire login, controlli anti-bot, DRM e restrizioni geografiche. L'app non configura attualmente cookie o un flusso di login.

## Aggiornamento strumenti audio

Usa lo stesso metodo dell'installazione originale. Per installazioni WinGet:

```powershell
winget upgrade --id shinchiro.mpv --exact --source winget
```

```powershell
winget upgrade --id yt-dlp.yt-dlp --exact --source winget
```

```powershell
winget upgrade streamlink --source winget
```

## Problemi di build

| Sintomo | Azione |
| --- | --- |
| `link.exe` o librerie Windows mancanti | Installa workload C++ desktop e Windows SDK dai prerequisiti Tauri. |
| WebView2 mancante | Installa WebView2 Runtime. |
| Porta 1420 occupata | Chiudi la precedente sessione di sviluppo prima di avviarne un'altra. |
| Errore di compilazione `EndEvent: Clone` | Usa il sorgente attuale: `EndEvent` deriva `Serialize` e `Clone` per `emit` di Tauri. |
| Errore dipendenze `npm ci` | Usa Node LTS supportato e il `package-lock.json` incluso; riporta l'errore concreto. |

Non esiste attualmente una suite automatica che verifichi streaming e tray. Riporta comando fallito, errore rilevante, versione Windows e versioni degli strumenti. Rimuovi URL privati e token dai log condivisi.

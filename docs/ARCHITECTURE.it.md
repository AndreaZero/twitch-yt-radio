# Architettura

[English](ARCHITECTURE.en.md) | [Italiano](ARCHITECTURE.it.md) | [Home](../README.it.md)

## Componenti

| File | Responsabilità |
| --- | --- |
| `src/App.tsx` | UI React, inserimento URL, playlist, persistenza locale, diagnostica, eventi di riproduzione. |
| `src/VideoPanel.tsx` | Misurazione area video, controlli visualizzazione, UI staccata e controlli riproduzione sincronizzati. |
| `src/styles.css` | Layout desktop compatto. |
| `src-tauri/src/lib.rs` | Comandi Rust validati, ciclo dei processi, risoluzione Twitch, IPC MPV, tray. |
| `src-tauri/src/playback.rs` | Modalità validate e opzioni per formati/input. |
| `src-tauri/src/video_host.rs` | Host figlio Windows, coordinate fisiche validate e limitate, fullscreen e cambio parent fra finestre dell'app. |
| `src-tauri/src/dependencies.rs` | Risoluzione e verifica dei percorsi eseguibili, incluse le cartelle di installazione Windows. |
| `src-tauri/src/main.rs` | Punto di ingresso nativo. |
| `src-tauri/capabilities/default.json` | Configurazione capability Tauri; nessun plugin shell nel frontend. |
| `src-tauri/tauri.conf.json` | Finestra, content security policy, integrazione Vite, packaging NSIS. |

## Flusso di riproduzione

1. React valida l'URL e invoca `start_playback` con URL, volume e modalità (`audio_only` o `video_audio`). Senza modalità viene usato solo audio; valori sconosciuti vengono rifiutati dalla deserializzazione.
2. Rust verifica HTTPS, rifiuta credenziali incorporate, limita la lunghezza e accetta solo gli hostname YouTube/Twitch configurati.
3. La modalità audio usa `bestaudio` YouTube oppure `audio_only` Streamlink, senza fallback video. La modalità video usa `bestvideo[height<=720]+bestaudio/best[height<=720]` YouTube oppure `best` Twitch.
4. Rust avvia MPV senza console. La modalità audio disabilita il video; quella video passa `--wid=<uint32 HWND>` per un host figlio nativo dell'app sopra il placeholder React. Le operazioni native avvengono nel thread UI Tauri. React invia soltanto dimensioni e intenzioni di visualizzazione, mai handle di finestre o comandi shell. Binding predefiniti, input tastiera/cursore e OSC MPV sono disabilitati per mantenere i controlli in React.
5. Pausa e volume viaggiano come JSON su una named pipe Windows locale. Stop termina il processo figlio.
6. Un monitor controlla l'uscita MPV ed emette `playback-ended`. Un contatore di generazione esclude eventi obsoleti dopo cambio traccia o stop.
7. React avanza soltanto dopo un'uscita senza errori. In caso di errore mostra il messaggio e non salta automaticamente. Gli eventi di fine ricevuti durante l'avvio vengono differiti fino al risultato e verificati rispetto alla generazione corrente. L'area nativa viene nascosta al termine soltanto se la generazione è ancora quella corrente.

La webview `video` viene creata nascosta insieme alla finestra principale. Distacco e reinserimento cambiano il parent dell'host dell'app fra queste due finestre tramite Win32 `SetParent`; MPV ne rimane figlio e continua la riproduzione. Lo schermo intero usa la finestra Tauri proprietaria e un layout React con barra di controllo visibile. La chiusura della finestra staccata viene intercettata e reinserisce il video. Le coordinate usano device pixel ratio, limiti dell'area client e revisioni per rifiutare aggiornamenti fuori ordine. La finestra inattiva non può spostare/nascondere l'area attiva. Eventi DPI/resize provocano nuove misurazioni. Le impostazioni principali nascondono il video integrato mantenendo la riproduzione. Non sono previsti decodifica multimediale nel browser o proxy locale.

Il cambio modalità riapre l'URL corrente con lo stesso comando di avvio, azzera la pausa, riavvia YouTube dall'inizio e riconnette Twitch alla diretta. Errori di risoluzione dipendenze/sorgente e avvio del processo lasciano intatto il player esistente. Un successivo errore asincrono MPV ferma la riproduzione. Non sono previsti cambio continuo o ripristino della posizione.

Pausa, volume, avvio e stop emettono snapshot registrati per sincronizzare i controlli delle due webview. Non c'è conferma dello stato tramite IPC MPV bidirezionale: pausa e volume riflettono ancora il comando registrato localmente. L'app non rileva attualmente buffering o audio effettivamente udibile.

## Stato e ciclo di vita

- Rust possiede il processo MPV in `Arc<Mutex<Player>>`.
- Playlist, volume e modalità preferita risiedono in `localStorage` WebView2; nessuna sincronizzazione cloud. Dopo un reload UI prevale lo snapshot del motore attivo.
- Chiudere la finestra principale nasconde anche il video integrato. Il video staccato resta visibile; chiuderlo lo reinserisce e mostra il widget. Stop o modalità audio chiudono il popout ed escono da schermo intero. L'avanzamento normale della playlist conserva la posizione del video. Esci dalla tray termina MPV e l'app.
- A ogni avvio il video parte integrato. La modalità audio/video resta salvata; una preferenza audio esistente non viene cambiata silenziosamente in video.
- Gli identificatori interni esistenti (`background-audio`, `background_audio_lib`, `dev.andreazero.backgroundaudio`) sono mantenuti. Il nome della repository pubblica è `twitch-yt-radio`.
- Il packaging non include eseguibili delle dipendenze audio e non pubblica release.

## Vincoli di sviluppo

Mantieni separate UI React e motore Rust/MPV. Non esporre shell nel frontend e non sostituire silenziosamente `audio_only` Twitch con video. Preferisci modifiche piccole e isolate ed errori concreti. Non avanzare automaticamente dopo errori di riproduzione. Evita dipendenze inutili e loop ripetitivi di build/test. Documenta assunzioni e verifiche Windows reali, senza dichiarare funzionante ciò che non hai eseguito.

Le istruzioni locali originali rimangono in [AGENTS.md](../AGENTS.md). Il processo bilingue per contribuire è in [CONTRIBUTING.md](../CONTRIBUTING.md).

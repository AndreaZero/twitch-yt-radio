# Architettura

[English](ARCHITECTURE.en.md) | [Italiano](ARCHITECTURE.it.md) | [Home](../README.it.md)

## Componenti

| File | Responsabilità |
| --- | --- |
| `src/App.tsx` | UI React, inserimento URL, playlist, persistenza locale, diagnostica, eventi di riproduzione. |
| `src/styles.css` | Layout desktop compatto. |
| `src-tauri/src/lib.rs` | Comandi Rust validati, ciclo dei processi, risoluzione Twitch, IPC MPV, tray. |
| `src-tauri/src/main.rs` | Punto di ingresso nativo. |
| `src-tauri/capabilities/default.json` | Configurazione capability Tauri; nessun plugin shell nel frontend. |
| `src-tauri/tauri.conf.json` | Finestra, content security policy, integrazione Vite, packaging NSIS. |

## Flusso di riproduzione

1. React valida l'URL e invoca `start_playback` con URL e volume.
2. Rust verifica HTTPS, rifiuta credenziali incorporate, limita la lunghezza e accetta solo gli hostname YouTube/Twitch configurati.
3. YouTube passa a MPV con `--ytdl-format=bestaudio`; Twitch viene risolto con `streamlink --stream-url URL audio_only`.
4. Rust avvia MPV senza finestra video o console. Gli eseguibili esterni sono chiamati direttamente con argomenti, senza stringhe shell.
5. Pausa e volume viaggiano come JSON su una named pipe Windows locale. Stop termina il processo figlio.
6. Un monitor controlla l'uscita MPV ed emette `playback-ended`. Un contatore di generazione esclude eventi obsoleti dopo cambio traccia o stop.
7. React avanza soltanto dopo un'uscita senza errori. In caso di errore mostra il messaggio e non salta automaticamente.

Non c'è conferma dello stato tramite IPC bidirezionale: pausa e volume riflettono il comando registrato localmente. L'app non rileva attualmente buffering o audio effettivamente udibile.

## Stato e ciclo di vita

- Rust possiede il processo MPV in `Arc<Mutex<Player>>`.
- Playlist e volume risiedono in `localStorage` WebView2; nessuna sincronizzazione cloud.
- Chiudere la finestra la nasconde. Esci dalla tray termina MPV e l'app.
- Gli identificatori interni esistenti (`background-audio`, `background_audio_lib`, `dev.andreazero.backgroundaudio`) sono mantenuti. Il nome della repository pubblica è `twitch-yt-radio`.
- Il packaging non include eseguibili delle dipendenze audio e non pubblica release.

## Vincoli di sviluppo

Mantieni separate UI React e motore Rust/MPV. Non esporre shell nel frontend e non sostituire silenziosamente `audio_only` Twitch con video. Preferisci modifiche piccole e isolate ed errori concreti. Non avanzare automaticamente dopo errori di riproduzione. Evita dipendenze inutili e loop ripetitivi di build/test. Documenta assunzioni e verifiche Windows reali, senza dichiarare funzionante ciò che non hai eseguito.

Le istruzioni locali originali rimangono in [AGENTS.md](../AGENTS.md). Il processo bilingue per contribuire è in [CONTRIBUTING.md](../CONTRIBUTING.md).

# Architettura

[English](ARCHITECTURE.en.md) | [Italiano](ARCHITECTURE.it.md) | [Home](../README.it.md)

## Componenti

| File | Responsabilità |
| --- | --- |
| `src/App.tsx` | UI React, inserimento URL, playlist, persistenza locale, diagnostica, eventi di riproduzione. |
| `src/styles.css` | Layout desktop compatto. |
| `src-tauri/src/lib.rs` | Comandi Rust validati, ciclo dei processi, risoluzione Twitch, IPC MPV, tray. |
| `src-tauri/src/playback.rs` | Modalità validate, selezione formati/finestra e riconoscimento della chiusura video manuale. |
| `src-tauri/src/dependencies.rs` | Risoluzione e verifica dei percorsi eseguibili, incluse le cartelle di installazione Windows. |
| `src-tauri/src/main.rs` | Punto di ingresso nativo. |
| `src-tauri/capabilities/default.json` | Configurazione capability Tauri; nessun plugin shell nel frontend. |
| `src-tauri/tauri.conf.json` | Finestra, content security policy, integrazione Vite, packaging NSIS. |

## Flusso di riproduzione

1. React valida l'URL e invoca `start_playback` con URL, volume e modalità (`audio_only` o `video_audio`). Senza modalità viene usato solo audio; valori sconosciuti vengono rifiutati dalla deserializzazione.
2. Rust verifica HTTPS, rifiuta credenziali incorporate, limita la lunghezza e accetta solo gli hostname YouTube/Twitch configurati.
3. La modalità audio usa `bestaudio` YouTube oppure `audio_only` Streamlink, senza fallback video. La modalità video usa `bestvideo[height<=720]+bestaudio/best[height<=720]` YouTube oppure `best` Twitch.
4. Rust avvia MPV senza console, nascondendo il video in modalità audio oppure aprendo una mini finestra separata sempre in primo piano (`--autofit=384x216`). Gli eseguibili sono chiamati direttamente con argomenti, senza shell. Una configurazione fissa nella cache dell'app associa `CLOSE_WIN` a `quit 10`; binding predefiniti e OSC sono disabilitati per mantenere i controlli in React.
5. Pausa e volume viaggiano come JSON su una named pipe Windows locale. Stop termina il processo figlio.
6. Un monitor controlla l'uscita MPV ed emette `playback-ended`. Un contatore di generazione esclude eventi obsoleti dopo cambio traccia o stop.
7. React avanza soltanto dopo un'uscita senza errori. In caso di errore mostra il messaggio e non salta automaticamente. Il codice 10 in modalità video indica chiusura manuale: ferma senza errore o avanzamento. Gli eventi di fine ricevuti durante l'avvio vengono differiti fino al risultato e verificati rispetto alla generazione corrente.

Il cambio modalità riapre l'URL corrente con lo stesso comando di avvio, azzera la pausa, riavvia YouTube dall'inizio e riconnette Twitch alla diretta. Errori di risoluzione dipendenze/sorgente e avvio del processo lasciano intatto il player esistente. Un successivo errore asincrono MPV ferma la riproduzione. Non sono previsti cambio continuo o ripristino della posizione.

Non c'è conferma dello stato tramite IPC bidirezionale: pausa e volume riflettono il comando registrato localmente. L'app non rileva attualmente buffering o audio effettivamente udibile.

## Stato e ciclo di vita

- Rust possiede il processo MPV in `Arc<Mutex<Player>>`.
- Playlist, volume e modalità preferita risiedono in `localStorage` WebView2; nessuna sincronizzazione cloud. Dopo un reload UI prevale lo snapshot del motore attivo.
- Chiudere la finestra principale la nasconde. La finestra video separata resta visibile finché viene chiusa, arrestata o si passa a solo audio. Esci dalla tray termina MPV e l'app.
- Gli identificatori interni esistenti (`background-audio`, `background_audio_lib`, `dev.andreazero.backgroundaudio`) sono mantenuti. Il nome della repository pubblica è `twitch-yt-radio`.
- Il packaging non include eseguibili delle dipendenze audio e non pubblica release.

## Vincoli di sviluppo

Mantieni separate UI React e motore Rust/MPV. Non esporre shell nel frontend e non sostituire silenziosamente `audio_only` Twitch con video. Preferisci modifiche piccole e isolate ed errori concreti. Non avanzare automaticamente dopo errori di riproduzione. Evita dipendenze inutili e loop ripetitivi di build/test. Documenta assunzioni e verifiche Windows reali, senza dichiarare funzionante ciò che non hai eseguito.

Le istruzioni locali originali rimangono in [AGENTS.md](../AGENTS.md). Il processo bilingue per contribuire è in [CONTRIBUTING.md](../CONTRIBUTING.md).

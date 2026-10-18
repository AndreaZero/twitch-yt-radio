# Contributing / Contribuire

[English](#english) | [Italiano](#italiano)

## English

Open an [issue](https://github.com/AndreaZero/twitch-yt-radio/issues) for a concrete bug or proposed change. Include reproduction steps, expected/actual behavior, Windows and dependency versions, and sanitized error output. Submit small pull requests explaining the problem, change, and checks actually run. Update both documentation languages when changing setup or user-facing behavior.

Read [architecture and constraints](docs/ARCHITECTURE.en.md). Preserve the separation between React and validated Rust commands, audio-only behavior, and stopping automatic queue advancement on errors. Avoid unnecessary dependencies and repeated build/test cycles.

Choose a focused check appropriate to the change. `npm run build` checks TypeScript and builds the frontend; it does not verify Rust or playback. For Rust changes, use a targeted Cargo check or the Windows Tauri build as appropriate. Do not call browser-only preview a native app test. This project has no automated streaming test suite or release workflow.

Before claiming Windows playback works, record which of these manual checks you actually performed:

1. Fresh build/installer starts with WebView2 available.
2. Each dependency is detected through successful `--version` execution.
3. A public YouTube video plays audio without a video window.
4. A live Twitch channel with `audio_only` plays; a channel without it reports an error without video fallback.
5. Pause/resume, volume/mute, stop, previous/next behave correctly.
6. Normal completion advances; an error does not skip automatically.
7. Closing hides to tray; reopening works; **Esci** stops MPV and exits.
8. Queue and volume persist on restart; no orphan MPV process remains after normal Quit.
9. In **Video + audio**, YouTube and Twitch render inside the widget with sound and no separate player window by default. Check pause/resume, volume and stop in both modes.
10. Switch modes while playing or paused: YouTube restarts, Twitch reconnects, video disappears in audio mode, and playback resumes unpaused. Audio mode still refuses Twitch channels without `audio_only`.
    Stop from the detached window during source resolution must cancel the pending start rather than resume later.
11. Detach, resize, reinsert and close the popout during playback, pause and buffering: the same stream and position should continue. Check pause, volume and stop from both windows. Fullscreen should keep an exit button and controls visible, then restore compact size on exit/Stop/audio mode, including after EOF/error. Escape works when the webview has keyboard focus.
12. Mode survives app restart; video placement starts embedded. UI reload restores backend mode, placement and volume. A real EOF advances in the current placement; an error stops. Quit leaves no MPV window/process. Check queue scrolling at 370 × 510 and default size; test 100%, 150%, 200% display scaling and moves between monitors. Embedded video must remain within its area, hide in settings/tray, and never cover the control strip.

## Italiano

Apri una [issue](https://github.com/AndreaZero/twitch-yt-radio/issues) per un bug concreto o una proposta. Includi passi per riprodurre, comportamento previsto/effettivo, versioni Windows e dipendenze ed errori ripuliti da dati privati. Invia pull request piccole che spieghino problema, modifica e verifiche realmente eseguite. Aggiorna entrambe le lingue quando cambiano configurazione o comportamento visibile.

Leggi [architettura e vincoli](docs/ARCHITECTURE.it.md). Conserva la separazione tra React e comandi Rust validati, la riproduzione solo audio e il blocco dell'avanzamento automatico in caso di errore. Evita dipendenze superflue e cicli ripetitivi di build/test.

Scegli una verifica mirata alla modifica. `npm run build` controlla TypeScript e compila il frontend; non verifica Rust o riproduzione. Per Rust usa un controllo Cargo mirato o la build Windows Tauri, secondo necessità. La preview nel browser non è un test dell'app nativa. Non ci sono suite automatiche di streaming o workflow di release.

Prima di dichiarare funzionante la riproduzione Windows, indica quali controlli manuali hai davvero eseguito:

1. Nuova build/installer si avvia con WebView2 disponibile.
2. Ogni dipendenza è rilevata tramite esecuzione riuscita di `--version`.
3. Un video YouTube pubblico riproduce audio senza finestra video.
4. Un canale Twitch live con `audio_only` funziona; senza questo flusso viene mostrato un errore senza fallback video.
5. Pausa/ripresa, volume/mute, stop e precedente/successivo funzionano.
6. Il termine normale avanza; un errore non salta automaticamente.
7. La chiusura nasconde nella tray, la riapertura funziona, **Esci** termina MPV e l'app.
8. Playlist e volume persistono al riavvio; nessun processo MPV orfano dopo l'uscita normale.
9. In **Video + audio**, YouTube e Twitch mostrano il video dentro il widget con audio, senza finestra player separata per impostazione predefinita. Verifica pausa/ripresa, volume e stop in entrambe le modalità.
10. Cambia modalità durante riproduzione e pausa: YouTube riparte, Twitch si riconnette, il video scompare passando a solo audio e la riproduzione riprende senza pausa. La modalità audio continua a rifiutare Twitch senza `audio_only`.
    Stop dalla finestra staccata durante la risoluzione della sorgente deve annullare l'avvio in corso, senza ripartire dopo.
11. Stacca, ridimensiona, reinserisci e chiudi il popout durante riproduzione, pausa e buffering: stream e posizione devono continuare. Verifica pausa, volume e stop da entrambe le finestre. Schermo intero deve mantenere pulsante di uscita e controlli visibili e ripristinare le dimensioni compatte all'uscita/Stop/modalità audio, anche dopo fine/errore. Esc funziona quando la webview ha il focus della tastiera.
12. La modalità resta salvata al riavvio; il video parte integrato. Il reload UI ripristina modalità, posizione e volume del motore. La fine naturale avanza nella posizione attuale; un errore ferma. Esci non lascia finestre/processi MPV. Verifica scorrimento playlist a 370 × 510 e dimensioni predefinite; prova scaling schermo 100%, 150%, 200% e cambio monitor. Il video integrato deve restare nella propria area, nascondersi in impostazioni/tray e non coprire la barra dei controlli.

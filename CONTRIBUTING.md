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
9. In **Video + audio**, YouTube and Twitch open one compact, always-on-top MPV window with sound; widget size stays unchanged. Check pause/resume, volume and stop in both modes.
10. Switch modes while playing or paused: YouTube restarts, Twitch reconnects, video disappears in audio mode, and playback resumes unpaused. Audio mode still refuses Twitch channels without `audio_only`.
11. Closing the video window stops without advancing or showing a playback error. A real EOF advances in the selected mode; an error stops. Check closing during buffering too.
12. Mode survives app restart; UI reload restores the active backend mode and volume. Quit with video active leaves no MPV window/process. Check queue scrolling at 370 × 510 and the default widget size.

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
9. In **Video + audio**, YouTube e Twitch aprono una sola finestra MPV piccola, sempre in primo piano e con audio; dimensioni del widget invariate. Verifica pausa/ripresa, volume e stop in entrambe le modalità.
10. Cambia modalità durante riproduzione e pausa: YouTube riparte, Twitch si riconnette, il video scompare passando a solo audio e la riproduzione riprende senza pausa. La modalità audio continua a rifiutare Twitch senza `audio_only`.
11. Chiudere il video ferma senza avanzare o mostrare un errore di riproduzione. La fine naturale avanza nella modalità selezionata; un errore ferma. Verifica la chiusura anche durante il buffering.
12. La modalità resta salvata al riavvio; il reload UI ripristina modalità e volume del motore attivo. Esci con video attivo non lascia finestre/processi MPV. Verifica lo scorrimento playlist a 370 × 510 e alle dimensioni predefinite.

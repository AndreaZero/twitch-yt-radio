# twitch-yt-radio

[English](README.md) | [Italiano](README.it.md)

Un player desktop compatto per Windows 11 per **YouTube e Twitch, con modalità solo audio o video opzionale**, realizzato con Tauri 2, React, TypeScript, Rust e MPV.

**Stato:** MVP personale, versione 0.1.0. Solo codice sorgente: non ci sono release pubblicate o installer da scaricare. Puoi creare un installer Windows locale seguendo la [guida di installazione](docs/INSTALL.it.md).

L'applicazione attuale si presenta come **Background Audio**. Interfaccia ed errori di riproduzione sono attualmente in italiano; la documentazione è disponibile in inglese e italiano.

## Funzionalità

- Player in vetro scuro, con barra canale/link e viste separate Player, Libreria e Impostazioni. Controlli video in sovraimpressione al movimento del mouse, nascosti dopo 2 secondi di inattività; chat sotto il player per le dirette Twitch.
- Aggiunta di singoli video YouTube o canali Twitch live a una playlist locale, anche scrivendo soltanto il nome del canale Twitch.
- Chat Twitch live con lettura e invio messaggi dal proprio account, accesso tramite codice nel browser. Richiede il Client ID di un'app Twitch pubblica: [configurazione e limiti](docs/TWITCH-CHAT.it.md).
- Chat staccabile in finestra; scorrimento live con pausa in hover o durante la lettura. Account Twitch ricordato tramite Gestione credenziali Windows e rinnovo dei token.
- [Valutazione della chat YouTube](docs/YOUTUBE-CHAT.it.md): integrazione separata, non ancora implementata.
- Scelta **Solo audio** (predefinita) o **Video + audio** nel widget, salvata localmente.
- In modalità audio, YouTube richiede `bestaudio`; Twitch richiede `audio_only` a Streamlink e segnala un errore se non disponibile, senza fallback video.
- La modalità video mostra MPV **dentro il widget per impostazione predefinita**, con finestra iniziale 450 × 540 e controlli compatti. Il vetro usa Acrylic Windows e dipende dalle impostazioni di trasparenza del sistema.
- Le icone accanto al selettore permettono **schermo intero** e **distacco/reinserimento** del player. Queste operazioni mantengono lo stesso stream, la posizione e la pausa. A schermo intero resta una barra di controllo con il pulsante per uscire; anche Esc esce quando la webview ha il focus della tastiera.
- La finestra staccata opzionale è ridimensionabile e sempre in primo piano, con pausa, stop, volume, schermo intero e reinserimento. Chiuderla riporta il video nel widget continuando la riproduzione. Ogni avvio dell'app parte con video integrato; distacco e schermo intero non sono salvati.
- La modalità video richiede video YouTube fino a 720p con audio, oppure il flusso Twitch `best`. Il video Twitch può consumare molta più banda.
- Cambiare modalità durante la riproduzione riapre la sorgente: YouTube riparte dall'inizio e Twitch si riconnette alla diretta. La riproduzione riprende senza pausa. Il cambio non è continuo.
- Play, pausa, stop, precedente/successivo, mute e volume.
- Passaggio alla voce successiva dopo la normale uscita di MPV; nessun avanzamento automatico in caso di errore. Per continuare ad ascoltare senza video scegli **Solo audio**; Stop termina la riproduzione e torna alla vista compatta.
- Playlist e volume persistenti nello storage locale di WebView2.
- Chiusura della finestra nella tray di Windows; **Esci** termina l'app e il suo processo MPV.
- Diagnostica delle dipendenze dal pannello impostazioni.

## Avvio rapido

Installa i [prerequisiti Windows e le dipendenze audio](docs/INSTALL.it.md), poi esegui ogni comando separatamente in PowerShell:

```powershell
git clone https://github.com/AndreaZero/twitch-yt-radio.git
```

```powershell
cd twitch-yt-radio
```

```powershell
npm ci
```

```powershell
npm run tauri dev
```

Per creare un installer NSIS locale:

```powershell
npm run tauri build
```

Output installer: `src-tauri\target\release\bundle\nsis\`. **MPV, yt-dlp e Streamlink non sono inclusi** e devono restare disponibili nel PATH dell'app.

## Documentazione

| Argomento | English | Italiano |
| --- | --- | --- |
| Installazione, sviluppo, installer locale | [Guide](docs/INSTALL.en.md) | [Guida](docs/INSTALL.it.md) |
| Problemi di PATH, riproduzione e build | [Guide](docs/TROUBLESHOOTING.en.md) | [Guida](docs/TROUBLESHOOTING.it.md) |
| Architettura e regole di sviluppo | [Guide](docs/ARCHITECTURE.en.md) | [Guida](docs/ARCHITECTURE.it.md) |
| Contributi e verifiche Windows | [Contributing](CONTRIBUTING.md#english) | [Contribuire](CONTRIBUTING.md#italiano) |
| Sicurezza e privacy | [Policy](SECURITY.md#english) | [Informazioni](SECURITY.md#italiano) |

## Limiti noti

- Piattaforma prevista: Windows 11; i controlli usano named pipe Windows. macOS e Linux non sono supportati da questo MVP.
- La modalità audio Twitch richiede un canale live che esponga `audio_only`; quella video richiede esplicitamente un flusso video.
- Login, verifiche anti-bot, DRM e restrizioni geografiche YouTube possono impedire la riproduzione. Gli strumenti esterni possono richiedere aggiornamenti quando cambiano le piattaforme.
- Le voci sono link singoli; importazione di playlist YouTube, titoli completi e copertine non sono implementati.
- Mancano tasti multimediali globali, riconnessione live, retry e aggiornamento automatico delle dipendenze.
- Un processo MPV attivo non dimostra che l'audio sia udibile o il buffering terminato. L'output MPV è attualmente soppresso.
- Nessuna funzione di esportazione o download di file media.

La sessione di configurazione ha confermato il rilevamento degli eseguibili sul PC Windows dell'utente. La preparazione della repository non certifica streaming completo, controlli o una nuova build dell'installer. Vedi la [checklist manuale Windows](CONTRIBUTING.md#italiano).

Rendering integrato, distacco/reinserimento, schermo intero, cambi DPI e cambio audio/video richiedono ancora la prova manuale su Windows; l'app non viene avviata durante la verifica del codice. L'integrazione usa il supporto HWND nativo del [manuale ufficiale MPV](https://mpv.io/manual/stable/#options-wid), senza player multimediale nel browser. Il piccolo host Windows usa `windows-sys`, già presente nelle dipendenze transitive di Tauri.

Progetto indipendente da Twitch e YouTube. Usalo nel rispetto delle condizioni dei servizi e dei diritti sui contenuti. Gli strumenti esterni mantengono le proprie licenze; qui non ne vengono redistribuiti gli eseguibili.

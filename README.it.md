# twitch-yt-radio

[English](README.md) | [Italiano](README.it.md)

Un player desktop compatto per Windows 11 per **YouTube e Twitch, con modalità solo audio o video opzionale**, realizzato con Tauri 2, React, TypeScript, Rust e MPV.

**Stato:** MVP personale, versione 0.1.0. Solo codice sorgente: non ci sono release pubblicate o installer da scaricare. Puoi creare un installer Windows locale seguendo la [guida di installazione](docs/INSTALL.it.md).

L'applicazione attuale si presenta come **Background Audio**. Interfaccia ed errori di riproduzione sono attualmente in italiano; la documentazione è disponibile in inglese e italiano.

## Funzionalità

- Aggiunta di singoli video YouTube o canali Twitch live a una playlist locale.
- Scelta **Solo audio** (predefinita) o **Video + audio** nel widget, salvata localmente.
- In modalità audio, YouTube richiede `bestaudio`; Twitch richiede `audio_only` a Streamlink e segnala un errore se non disponibile, senza fallback video.
- La modalità video apre una mini finestra MPV separata, ridimensionabile e sempre in primo piano, inizialmente entro 384 × 216 pixel (area video). Il widget mantiene le dimensioni; pausa, volume e stop restano lì.
- La modalità video richiede video YouTube fino a 720p con audio, oppure il flusso Twitch `best`. Il video Twitch può consumare molta più banda.
- Cambiare modalità durante la riproduzione riapre la sorgente: YouTube riparte dall'inizio e Twitch si riconnette alla diretta. La riproduzione riprende senza pausa. Il cambio non è continuo.
- Play, pausa, stop, precedente/successivo, mute e volume.
- Passaggio alla voce successiva dopo la normale uscita di MPV; nessun avanzamento automatico in caso di errore o chiusura della finestra video. Per continuare ad ascoltare senza video, scegli **Solo audio** invece di chiudere MPV.
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

Mini finestra video e cambio modalità richiedono ancora la prova manuale della riproduzione su Windows; l'app non viene avviata durante la verifica del codice. Dimensionamento e gestione della chiusura MPV seguono il [manuale ufficiale MPV](https://mpv.io/manual/stable/).

Progetto indipendente da Twitch e YouTube. Usalo nel rispetto delle condizioni dei servizi e dei diritti sui contenuti. Gli strumenti esterni mantengono le proprie licenze; qui non ne vengono redistribuiti gli eseguibili.

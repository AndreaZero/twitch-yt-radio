# twitch-yt-radio

[English](README.md) | [Italiano](README.it.md)

Un player desktop compatto per Windows 11 per ascoltare **YouTube e Twitch senza una finestra video**, realizzato con Tauri 2, React, TypeScript, Rust e MPV.

**Stato:** MVP personale, versione 0.1.0. Solo codice sorgente: non ci sono release pubblicate o installer da scaricare. Puoi creare un installer Windows locale seguendo la [guida di installazione](docs/INSTALL.it.md).

L'applicazione attuale si presenta come **Background Audio**. Interfaccia ed errori di riproduzione sono attualmente in italiano; la documentazione è disponibile in inglese e italiano.

## Funzionalità

- Aggiunta di singoli video YouTube o canali Twitch live a una playlist locale.
- YouTube richiede `bestaudio` tramite MPV e yt-dlp.
- Twitch richiede a Streamlink `audio_only` e segnala un errore quando non è disponibile; nessun fallback automatico video.
- Play, pausa, stop, precedente/successivo, mute e volume.
- Passaggio alla voce successiva dopo la normale uscita di MPV; nessun avanzamento automatico in caso di errore.
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
- Twitch richiede un canale live che esponga `audio_only`.
- Login, verifiche anti-bot, DRM e restrizioni geografiche YouTube possono impedire la riproduzione. Gli strumenti esterni possono richiedere aggiornamenti quando cambiano le piattaforme.
- Le voci sono link singoli; importazione di playlist YouTube, titoli completi e copertine non sono implementati.
- Mancano tasti multimediali globali, riconnessione live, retry e aggiornamento automatico delle dipendenze.
- Un processo MPV attivo non dimostra che l'audio sia udibile o il buffering terminato. L'output MPV è attualmente soppresso.
- Nessuna funzione di esportazione o download di file media.

La sessione di configurazione ha confermato il rilevamento degli eseguibili sul PC Windows dell'utente. La preparazione della repository non certifica streaming completo, controlli o una nuova build dell'installer. Vedi la [checklist manuale Windows](CONTRIBUTING.md#italiano).

Progetto indipendente da Twitch e YouTube. Usalo nel rispetto delle condizioni dei servizi e dei diritti sui contenuti. Gli strumenti esterni mantengono le proprie licenze; qui non ne vengono redistribuiti gli eseguibili.

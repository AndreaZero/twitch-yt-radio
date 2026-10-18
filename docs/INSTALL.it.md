# Installazione e build su Windows

[English](INSTALL.en.md) | [Italiano](INSTALL.it.md) | [Home](../README.it.md)

Non ci sono release pubblicate. Compila il sorgente localmente e avvia l'installer generato. I comandi sono per PowerShell su Windows 11: incolla ed esegui **un comando alla volta**.

## Usare l'app compilata: installer o EXE

Per usare l'app compilata non servono Git, Node, Python, Rust o Visual Studio. Questi strumenti servono soltanto allo sviluppo.

- **Installer NSIS:** installazione per utente; se manca WebView2, il setup scarica e apre il bootstrapper Microsoft. Dopo l'installazione avvia Background Audio: se mancano componenti multimediali si apre la configurazione guidata nelle impostazioni.
- **Solo EXE:** può essere avviato direttamente. Se WebView2 manca, una finestra nativa propone di scaricarlo da Microsoft e avviare l'installazione, verificando prima la firma Windows. Se download/installazione falliscono, viene aperta la pagina ufficiale con istruzioni per riprovare. Quando WebView2 è disponibile parte la stessa guida dell'app installata.
- **Installa mancanti:** installa in sequenza MPV (`shinchiro.mpv`), yt-dlp (`yt-dlp.yt-dlp`), Streamlink (`Streamlink.Streamlink`), FFmpeg/FFprobe (`Gyan.FFmpeg.Essentials`) e Deno (`DenoLand.Deno`), saltando ciò che è già utilizzabile. Gli installer sono quelli del catalogo WinGet, non copie incorporate. Il pulsante accetta le condizioni WinGet/pacchetti; Windows può richiedere autorizzazioni. Non vengono ignorati hash o controlli di sicurezza.
- Se WinGet manca, la guida apre **App Installer** Microsoft. Installalo/aggiornalo e premi **Rileva di nuovo**. Se Store o criteri aziendali impediscono l'installazione, ogni componente ha il proprio sito ufficiale e un campo per un eseguibile locale già estratto/installato. Per FFmpeg serve anche FFprobe nella stessa cartella.

Non occorre copiare variabili d'ambiente: l'app legge il PATH attuale e quello salvato in Windows, cerca le cartelle di installazione e passa un PATH dedicato ai processi multimediali. La guida salva eventuali percorsi personalizzati in `tools.json` nella cartella configurazione dell'app. La cartella `bin` accanto all'EXE è supportata; mantieni anche DLL e file accessori degli strumenti. Dopo un'installazione riuscita non serve riavviare il player. Gli installer esterni possono comunque modificare il PATH di Windows secondo le proprie regole.

Serve Internet per scaricare i componenti. L'EXE non è un pacchetto offline autosufficiente: configurazione, account e strumenti restano nel profilo/PC anche spostando l'EXE. Non chiudere l'app dalla tray durante l'installazione. Non esiste annullamento forzato di un installer in corso; dopo timeout controlla i processi Windows prima di riprovare. Gli aggiornamenti automatici dei tool non sono implementati.

Account Twitch/Google opzionali: la guida rimanda alle rispettive impostazioni, senza richiedere variabili d'ambiente per token o client secret.

Fonti: [installer Tauri](https://tauri.app/distribute/windows-installer/), [distribuzione WebView2 Microsoft](https://learn.microsoft.com/en-us/microsoft-edge/webview2/concepts/distribution), [installazione WinGet](https://learn.microsoft.com/en-us/windows/package-manager/winget/install), [requisiti yt-dlp](https://github.com/yt-dlp/yt-dlp#dependencies).

## Dimensionare player e chat

Trascina la barra fra video e chat/commenti, anche con touch. Le frecce su/giù la regolano da tastiera; Home/End raggiungono i limiti. Doppio clic o Invio ripristinano l'altezza automatica. La proporzione scelta viene ricordata e limitata in base allo spazio disponibile, mantenendo accessibili entrambi i pannelli. Video staccato/schermo intero usano il proprio spazio e non mostrano questo separatore.

Il limite precedente di 320 px è rimosso. L'altezza automatica cresce con la finestra e tiene conto di un video 16:9; MPV conserva le proporzioni effettive della sorgente. Un riquadro più largo del video può ancora avere bande laterali: trascina il divisore verso il basso per aumentare l'immagine senza deformarla o ritagliarla.

Questa modifica è solo sviluppo: **nessuna build, test, installazione di prova o avvio dell'app eseguiti**. Da verificare dall'utente su Windows: PC senza tool/WinGet/WebView2, annullamento e rete assente, installazioni con UAC, percorsi con spazi, riavvio e persistenza, riproduzione con Deno/FFmpeg, trascinamento oltre l'area nativa MPV, ridimensionamento/DPI, chat in finestra e schermo intero.

## Prerequisiti di sviluppo

- [Git per Windows](https://git-scm.com/downloads/win).
- [Node.js](https://nodejs.org/en/download) con npm: usa una versione LTS supportata (Node 22 o 24).
- [Rust tramite rustup](https://rustup.rs/): stable, toolchain MSVC adatta all'architettura Windows.
- [Visual Studio Build Tools](https://visualstudio.microsoft.com/visual-cpp-build-tools/): **Sviluppo di applicazioni desktop con C++**, includendo MSVC e un Windows SDK.
- [Microsoft Edge WebView2 Runtime](https://developer.microsoft.com/en-us/microsoft-edge/webview2/), normalmente già presente su Windows 11.

Segui i [prerequisiti ufficiali Tauri per Windows](https://v2.tauri.app/start/prerequisites/#windows). Gli strumenti di compilazione sono separati dalle dipendenze di riproduzione seguenti.

## Alternativa manuale per gli strumenti

Con Windows Package Manager, esegui separatamente:

```powershell
winget install --id shinchiro.mpv --exact --source winget
```

```powershell
winget install --id yt-dlp.yt-dlp --exact --source winget
```

```powershell
winget install --id Streamlink.Streamlink --exact --source winget
```

```powershell
winget install --id Gyan.FFmpeg.Essentials --exact --source winget
```

```powershell
winget install --id DenoLand.Deno --exact --source winget
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

I tool multimediali **non sono incorporati**: al primo avvio usa la guida descritta sopra. Firma del codice dell'app e aggiornamento automatico non sono configurati; Windows può mostrare un avviso per un installer locale non firmato. Questi comandi non pubblicano una release GitHub.

Se l'app eredita ancora il vecchio PATH, esci dalla tray, aggiorna il PATH come sopra e avviala dalla stessa sessione PowerShell nella cartella principale:

```powershell
& '.\src-tauri\target\release\background-audio.exe'
```

Verifica il nuovo installer e la riproduzione reale su Windows con la [checklist manuale](../CONTRIBUTING.md#italiano).

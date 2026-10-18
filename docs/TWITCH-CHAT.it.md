# Chat Twitch

La chat è separata da MPV. Segue il canale Twitch in riproduzione e si scollega su stop o passaggio a YouTube. La pausa audio lascia la chat in diretta. Gli errori chat non modificano la riproduzione né fanno avanzare la playlist.

## Configurazione

1. Apri **Impostazioni → Account Twitch** e il collegamento alla console sviluppatori.
2. Registra un'app in [Twitch Developer Console](https://dev.twitch.tv/console/apps): tipo client **Public**, categoria applicazione desktop, redirect `http://localhost`. Il flusso device code non usa il redirect. Twitch può richiedere la verifica dell'account sviluppatore.
3. Incolla il **Client ID**, non il Client Secret. Questo identificativo pubblico viene ricordato nelle preferenze; la sessione OAuth viene salvata separatamente in Gestione credenziali Windows.
4. Premi **Collega Twitch**, poi **Apri Twitch**. Verifica il codice nel browser e autorizza `user:read:chat` e `user:write:chat`. La password rimane nel sito Twitch.
5. Avvia un canale Twitch: la chat compare sotto il player audio/video. Scrivi nel campo messaggio e premi Invia o Invio.

Non serve un account bot dedicato né essere moderatore del canale con un token utente. Ban, timeout, limiti dell'account o revoche possono impedire l'accesso. Scollegare nell'app termina la sessione locale; per revocare il consenso usa le connessioni nelle impostazioni Twitch.

Gli accessi precedenti con sola lettura richiedono una nuova autorizzazione: **Impostazioni → Account Twitch → Abilita invio messaggi**. L'invio usa il proprio account, massimo 500 caratteri, ed è consentito soltanto quando la chat è connessa al canale corrente. Rifiuti AutoMod, ban o limiti Twitch vengono mostrati senza cancellare la bozza. Nessun invio viene ripetuto automaticamente; se l'esito è incerto, controlla la chat prima di riprovare. Il messaggio pubblicato compare tramite EventSub. Nelle sessioni Shared Chat, Twitch può distribuirlo anche agli altri canali della sessione secondo le proprie regole.

Il cambio canale o account azzera la bozza. Un invio ancora in preparazione viene annullato se la sessione cambia; un messaggio già trasmesso a Twitch non può essere ritirato cambiando canale.

I controlli video sono in sovraimpressione: finestre trasparenti di proprietà del player sono posizionate sopra l'host MPV. Appaiono entrando o muovendo il mouse sul video e spariscono dopo 2 secondi di inattività; restano visibili finché il mouse è sui pulsanti. Non sottraggono altezza all'immagine. Il comportamento vale anche per YouTube e per video staccato/fullscreen. L'overlay è nascosto con il player o quando un'altra applicazione prende il primo piano.

L'icona finestra nella testata della chat apre una finestra indipendente con lo stesso buffer, account e canale. Il player continua a selezionare il canale; la finestra chat non apre connessioni duplicate. Reinserisci o la X della finestra riportano la chat nel player. Il distacco non è ricordato al riavvio.

La lista segue sempre l'ultimo messaggio in modalità live, anche dopo un ridimensionamento. L'hover sui messaggi congela le nuove righe per leggerle; lasciando la lista riprende il live se non hai scorrimento arretrato. Lo scroll verso l'alto mantiene la pausa. Scorrere in fondo o premere **Torna al live** riaggancia il flusso. Durante la pausa il buffer live continua a ricevere e le azioni di moderazione vengono applicate anche alle righe congelate. Ogni vista conserva al massimo 200 righe congelate oltre alle 200 live; una pausa lunga non è uno storico completo.

## Limiti

- Access token e refresh token risiedono nel processo Rust e in Gestione credenziali Windows, con protezione del profilo Windows corrente; non sono in localStorage, file JSON o repository. Dopo il primo accesso con questa versione vengono ripristinati all'avvio, validati e rinnovati prima della scadenza. Scollega elimina la credenziale salvata e impedisce a un rinnovo già in corso di risalvarla. Revoca, scadenza del refresh token, errori dell'archivio Windows o rinnovi concorrenti da più istanze possono richiedere un nuovo accesso: usa una sola istanza dell'app.
- La chat si connette anche con la relativa vista chiusa, quando account e canale sono disponibili. Conserva gli ultimi 200 messaggi in memoria.
- Riceve nuovi messaggi dalla connessione in poi, senza storico o recupero di quelli persi durante una disconnessione.
- Nomi, colori e testo; emote come testo. Badge, emote grafiche, notifiche abbonamenti e replay VOD non sono inclusi.
- Gestisce cancellazione di messaggi, rimozione dei messaggi di un utente e pulizia della chat. Deduplicazione con buffer limitato.
- Errori di rete: riconnessione con attesa crescente. Accesso negato o revocato: intervento dell'utente. Il cambio canale ricrea la sessione EventSub.
- Chat live e riproduzione non sono sincronizzate: il buffering MPV/Streamlink può far anticipare la chat.

HTTPS e WebSocket sicuri sono gestiti in Rust. Il frontend non riceve token e non dispone di shell o apertura generica di URL. I comandi browser aprono soltanto la console Twitch o un URL di attivazione Twitch validato.

## Verifica Windows

Verifiche della precedente versione in sola lettura, del 18 ottobre 2026: build frontend (`npm run build`), 10 test Rust (`cargo test --offline --manifest-path src-tauri/Cargo.toml --lib`) e compilazione dell'app debug completate su Windows. Ispezione dell'icona PNG completata. Verifica visiva della finestra bloccata dai permessi computer-use; anteprima browser non raggiungibile. Login e chat reali non eseguiti senza Client ID/autorizzazione.

Le modifiche a overlay, finestra chat, pausa/scorrimento e memorizzazione account non sono state compilate né testate, su richiesta dell'utente. Verificare overlay (hover, inattività, resize/DPI, fullscreen, distacco, Alt-Tab e tray), chat (lettura, moderazione, ritorno live e popout) e account (chiusura completa, riapertura, rinnovo, rete assente, Scollega e successivo riavvio).

Compilazione e test locali non sostituiscono la prova con account reale. Servono un Client ID pubblico e l'autorizzazione dell'utente per verificare login, messaggi, cambio canale, stop, passaggio a YouTube, riconnessione di rete, scollegamento e moderazione.

Per la UI verificare ridimensionamento, trascinamento dalla maniglia in alto a sinistra, tray, video integrato nascosto nelle altre viste, distacco e schermo intero. L'onda audio è decorativa: indica lo stato registrato e non misura il segnale audio.

Riferimenti: [Device Code Grant](https://dev.twitch.tv/docs/authentication/getting-tokens-oauth/#device-code-grant-flow), [ricezione messaggi](https://dev.twitch.tv/docs/chat/send-receive-messages/#receiving-chat-messages), [WebSocket EventSub](https://dev.twitch.tv/docs/eventsub/handling-websocket-events/).

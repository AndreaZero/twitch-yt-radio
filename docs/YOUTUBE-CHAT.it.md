# Chat e commenti YouTube

[English](YOUTUBE-CHAT.en.md) | [Italiano](YOUTUBE-CHAT.it.md)

Implementazione sorgente del 18 ottobre 2026. **Nessuna build, test o prova account/API eseguiti per questa modifica**, su richiesta dell'utente.

## Collegare Google

1. In [Google Cloud](https://console.cloud.google.com/apis/credentials) seleziona un progetto e abilita **YouTube Data API v3**.
2. Configura la schermata consenso OAuth. In Testing aggiungi il tuo account agli utenti di test.
3. Crea un client OAuth di tipo **App desktop**. Copia Client ID e Client Secret nelle impostazioni dell'app; non usare un client Web o Twitch.
4. Premi **Collega Google**, scegli l'account/canale YouTube nel browser di sistema e autorizza l'accesso. Il callback locale scade dopo tre minuti.

L'app usa lo scope `youtube.force-ssl` e il flusso [OAuth desktop](https://developers.google.com/identity/protocols/oauth2/native-app). Serve un canale YouTube associato all'account; il nome mostrato è quello del canale, non l'email.

Token e client secret sono ricordati in Gestione credenziali Windows, voce `dev.andreazero.backgroundaudio/google`, separata da Twitch. Il rinnovo avviene quando serve una richiesta YouTube, senza reinserire il secret. Scollega elimina la sessione locale; la revoca remota si gestisce nelle connessioni dell'account Google. Revoche, policy Workspace o [scadenza dei refresh token in Testing dopo sette giorni](https://developers.google.com/identity/protocols/oauth2#expiration) possono richiedere un nuovo consenso.

## Comportamento

- Link video supportati: `watch?v=`, `youtu.be`, `/live/`, `/shorts/`, `/embed/`. Rust valida host e ID; un link a un canale non identifica una chat specifica.
- Durante la riproduzione, `videos.list` distingue chat attiva, diretta programmata/senza chat e video normale. Il pannello funziona anche in solo audio senza cambiare lo stream MPV.
- Live: lettura, invio fino a 200 caratteri, ultimi 200 messaggi, deduplicazione e rimozioni/moderazione ricevute dalle API. La lista è la stessa di Twitch: segue il fondo, si ferma in hover, durante selezione o scroll manuale e riprende con **Torna al live**. Fino a 200 righe congelate possono restare visibili durante la lettura.
- Video normale/registrazione: commenti principali dal più recente, 50 per pagina, massimo 1.000 caricati; aggiornamento manuale e pubblicazione di nuovi commenti (limite locale 10.000 caratteri). Conteggio Mi piace e risposte; lettura/invio delle risposte, modifica, eliminazione e reazioni restano su YouTube. Nessuno scorrimento automatico dei commenti durante la lettura.
- Finestra YouTube separata con sessione Rust condivisa. Chiuderla reinserisce il pannello.
- Invii espliciti, blocco dei doppi click e controllo di account/video prima del POST; nessun retry automatico. Un timeout può avere già pubblicato il testo: la bozza resta e l'errore invita a controllare. Moderazione e restrizioni YouTube possono impedire invio o visibilità immediata.

## Tempi, quota e limiti

Questa versione usa [liveChatMessages.list](https://developers.google.com/youtube/data/live/reference/rest/v3/liveChatMessages/list), attendendo almeno `pollingIntervalMillis`, con minimo locale di cinque secondi. Non è push come Twitch. La quota è condivisa dal progetto Google Cloud: con cinque unità per lettura ogni cinque secondi si consumano circa 3.600 unità/ora, oltre agli invii e alle altre richieste. Intervalli API maggiori riducono il consumo. Quota esaurita interrompe le letture con un errore; non ci sono tentativi continui. [streamList gRPC](https://developers.google.com/youtube/data/live/streaming-live-chat) è una possibile ottimizzazione futura, non implementata qui.

I [commenti](https://developers.google.com/youtube/data/reference/rest/v3/commentThreads/list) si caricano su richiesta. Dirette programmate/senza chat vengono ricontrollate circa ogni 30–60 secondi. Le letture con errori di rete ritentano con attesa crescente; permessi, quota e contenuto disabilitato richiedono intervento dell'utente. Nessun replay chat dopo la fine live o sincronizzazione con la posizione audio/video. L'accesso Google della community non autentica MPV/yt-dlp.

## Da verificare dall'utente su Windows

Accesso/annullamento browser, riapertura senza login, rinnovo/revoca, identità YouTube corretta, chat e invio singolo, hover/scroll/selezione/ritorno al fondo, moderazione, finestra separata, cambio rapido video/Twitch/stop durante richieste, commenti disabilitati, paginazione/pubblicazione, rete assente e quota esaurita. Prove da eseguire dopo la build dell'utente.

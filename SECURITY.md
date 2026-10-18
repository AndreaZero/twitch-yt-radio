# Security and privacy / Sicurezza e privacy

[English](#english) | [Italiano](#italiano)

## English

The frontend calls specific Rust commands. Rust validates supported HTTPS hosts and launches external programs without a shell. This is a security boundary, not a guarantee that remote content or third-party tools are safe.

MPV communicates through a local Windows named pipe, not a listening TCP server. The pipe name is unique per process/generation but has no application-level authentication; do not expose IPC to untrusted software in the same Windows session. There is no hosted backend, app telemetry, or cloud queue storage. URLs and volume remain in local WebView2 storage; streaming services and external tools receive the requests needed for playback.

Do not paste private URLs with tokens, credentials, or secrets into the queue or public issues. Keep MPV, yt-dlp, and Streamlink updated through their original installation method.

Twitch OAuth tokens are stored in Windows Credential Manager under `dev.andreazero.backgroundaudio/twitch`, scoped to the current Windows user, and never sent to React or written to source/config files. Disconnect deletes the stored session. Other software running as the same Windows user is outside this protection boundary. Twitch receives the chat requests and messages explicitly submitted by the user.

Google OAuth uses the system browser, random state, PKCE S256 and a temporary listener bound only to `127.0.0.1` on an ephemeral port (three-minute deadline). Google access/refresh tokens and the desktop client secret are stored separately under `dev.andreazero.backgroundaudio/google`. Tokens never enter React. The secret entered in settings is passed to Rust, cleared from the input and never saved in localStorage; only the public Client ID is saved there. Disconnect deletes local credentials; remote Google authorization can be revoked in Google account connections. Google receives selected video IDs, API requests and explicitly submitted public chat messages/comments. Public comments and live messages are displayed as text, never injected HTML. This is separate from authentication used by external playback tools.

For suspected vulnerabilities, use [GitHub private vulnerability reporting](https://github.com/AndreaZero/twitch-yt-radio/security/advisories/new) if enabled. If unavailable, open an issue asking for a private reporting channel without posting exploit details or secrets. This is a personal MVP with no promised response time or security maintenance window.

## Italiano

Il frontend chiama comandi Rust specifici. Rust valida gli host HTTPS supportati e avvia programmi esterni senza shell. È un confine di sicurezza, non una garanzia sulla sicurezza dei contenuti remoti o degli strumenti esterni.

MPV comunica tramite named pipe Windows locale, non un server TCP in ascolto. Il nome è unico per processo/generazione ma manca autenticazione a livello applicativo: non esporre l'IPC a software non fidato nella stessa sessione Windows. Non ci sono backend ospitato, telemetria dell'app o playlist cloud. URL e volume restano nello storage WebView2 locale; servizi streaming e strumenti esterni ricevono le richieste necessarie alla riproduzione.

Non inserire URL privati con token, credenziali o segreti nella playlist o nelle issue pubbliche. Mantieni aggiornati MPV, yt-dlp e Streamlink usando il metodo di installazione originale.

I token OAuth Twitch sono salvati in Gestione credenziali Windows con nome `dev.andreazero.backgroundaudio/twitch`, per il profilo Windows corrente. Non passano a React né a file sorgente/configurazione. Scollega elimina la sessione salvata. Il software che gira con lo stesso utente Windows resta fuori da questo confine di protezione. Twitch riceve le richieste chat e i messaggi inviati esplicitamente dall'utente.

Google OAuth usa il browser di sistema, state casuale, PKCE S256 e un listener temporaneo soltanto su `127.0.0.1`, porta effimera e scadenza di tre minuti. Token access/refresh e client secret desktop sono salvati separatamente in `dev.andreazero.backgroundaudio/google`. I token non arrivano a React. Il secret inserito nelle impostazioni viene passato a Rust e rimosso dal campo, mai salvato in localStorage; lì rimane soltanto il Client ID pubblico. Scollega elimina le credenziali locali; la revoca remota si gestisce nelle connessioni dell'account Google. Google riceve ID dei video selezionati, richieste API e messaggi/commenti pubblici inviati esplicitamente. I testi dei commenti e della chat non vengono inseriti come HTML. Questa autenticazione resta separata dagli strumenti esterni di riproduzione.

Per sospette vulnerabilità, usa la [segnalazione privata GitHub](https://github.com/AndreaZero/twitch-yt-radio/security/advisories/new) se abilitata. Se non disponibile, apri una issue chiedendo un canale privato senza pubblicare dettagli di exploit o segreti. È un MVP personale senza tempi di risposta o periodo di manutenzione di sicurezza garantiti.

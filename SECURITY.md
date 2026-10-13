# Security and privacy / Sicurezza e privacy

[English](#english) | [Italiano](#italiano)

## English

The frontend calls specific Rust commands. Rust validates supported HTTPS hosts and launches external programs without a shell. This is a security boundary, not a guarantee that remote content or third-party tools are safe.

MPV communicates through a local Windows named pipe, not a listening TCP server. The pipe name is unique per process/generation but has no application-level authentication; do not expose IPC to untrusted software in the same Windows session. There is no hosted backend, app telemetry, or cloud queue storage. URLs and volume remain in local WebView2 storage; streaming services and external tools receive the requests needed for playback.

Do not paste private URLs with tokens, credentials, or secrets into the queue or public issues. Keep MPV, yt-dlp, and Streamlink updated through their original installation method.

For suspected vulnerabilities, use [GitHub private vulnerability reporting](https://github.com/AndreaZero/twitch-yt-radio/security/advisories/new) if enabled. If unavailable, open an issue asking for a private reporting channel without posting exploit details or secrets. This is a personal MVP with no promised response time or security maintenance window.

## Italiano

Il frontend chiama comandi Rust specifici. Rust valida gli host HTTPS supportati e avvia programmi esterni senza shell. È un confine di sicurezza, non una garanzia sulla sicurezza dei contenuti remoti o degli strumenti esterni.

MPV comunica tramite named pipe Windows locale, non un server TCP in ascolto. Il nome è unico per processo/generazione ma manca autenticazione a livello applicativo: non esporre l'IPC a software non fidato nella stessa sessione Windows. Non ci sono backend ospitato, telemetria dell'app o playlist cloud. URL e volume restano nello storage WebView2 locale; servizi streaming e strumenti esterni ricevono le richieste necessarie alla riproduzione.

Non inserire URL privati con token, credenziali o segreti nella playlist o nelle issue pubbliche. Mantieni aggiornati MPV, yt-dlp e Streamlink usando il metodo di installazione originale.

Per sospette vulnerabilità, usa la [segnalazione privata GitHub](https://github.com/AndreaZero/twitch-yt-radio/security/advisories/new) se abilitata. Se non disponibile, apri una issue chiedendo un canale privato senza pubblicare dettagli di exploit o segreti. È un MVP personale senza tempi di risposta o periodo di manutenzione di sicurezza garantiti.

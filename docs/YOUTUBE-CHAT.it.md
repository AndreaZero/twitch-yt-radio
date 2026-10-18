# Chat YouTube: valutazione del 18 ottobre 2026

I nuovi controlli in sovraimpressione sono comuni a MPV e quindi si applicano anche ai video YouTube. Il player attuale non richiede un account Google per riprodurre i link pubblici supportati. La sessione Twitch non dà accesso a Google.

La chat YouTube riguarda le dirette con chat abilitata; non i commenti dei normali video. Il provider YouTube non è implementato in questa modifica.

Per una tranche dedicata:

- Ricavare `liveStreamingDetails.activeLiveChatId` dal video, senza mostrare un pannello chat per normali video, live senza chat o chat terminate.
- Ricevere i messaggi con `liveChatMessages.streamList`, usando il token di continuazione per riconnettersi. Google documenta autenticazione tramite API key oppure OAuth nella [guida streaming](https://developers.google.com/youtube/data/live/streaming-live-chat).
- Per scrivere, predisporre un progetto Google Cloud con YouTube Data API e OAuth desktop. L'endpoint [liveChatMessages.insert](https://developers.google.com/youtube/data/live/reference/rest/v3/liveChatMessages/insert) richiede consenso OAuth con uno degli scope YouTube previsti.
- Riutilizzare interfaccia, finestra chat e pausa di lettura; aggiungere un motore Rust dedicato a Google, con refresh token in una voce separata di Gestione credenziali Windows. Il flusso Google deve seguire le regole per [app desktop](https://developers.google.com/identity/protocols/oauth2/native-app), non il device code Twitch.
- Gestire quota del progetto, rinnovo/revoca account, moderazione e fine live. Attualmente Google indica 5 unità per [streamList](https://developers.google.com/youtube/data/live/reference/rest/v3/liveChatMessages/streamList) e 20 per l'invio; verificare questi valori quando si implementa.

La decisione proposta è riutilizzare il comportamento UI per entrambi i servizi, mantenendo connessione e autenticazione separate. Non ci sono build o prove con un account Google in questa valutazione.

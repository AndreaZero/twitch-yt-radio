# Regole di sviluppo – Background Audio

- Progetto personale Windows 11, evoluzione incrementale per task piccoli e isolati.
- Mantenere separati UI React e motore Rust/MPV.
- Non inserire una shell nel frontend: ogni chiamata esterna passa da comandi Rust validati.
- Non trasformare silenziosamente `audio_only` in streaming video.
- Non fare loop ripetitivi di build/test: una verifica mirata per task e indicare cosa serve testare su Windows.
- Evitare dipendenze non necessarie, preferire interfaccia compatta, messaggi di errore concreti.
- Se la riproduzione termina per errore, non saltare automaticamente alla successiva.
- Documentare limiti/assunzioni reali. Non dichiarare funzionante ciò che non è stato eseguito su Windows.

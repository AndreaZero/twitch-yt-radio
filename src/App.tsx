import { useEffect, useRef, useState } from 'react';
import type { CSSProperties } from 'react';
import { invoke } from '@tauri-apps/api/core';
import { listen } from '@tauri-apps/api/event';
import {
  AlertCircle, AudioLines, Check, ChevronDown, CircleHelp, Headphones,
  Link2, ListMusic, LoaderCircle, Pause, Play, Plus, Radio, RotateCcw,
  Settings2, SkipBack, SkipForward, Square, Trash2, Volume2, VolumeX,
  X,
} from 'lucide-react';

type Platform = 'youtube' | 'twitch';
type Track = { id: string; url: string; title: string; platform: Platform };
type Dependencies = { mpv: boolean; ytDlp: boolean; streamlink: boolean };
type Snapshot = {
  active: boolean;
  currentUrl: string | null;
  generation: number;
  paused: boolean;
  volume: number;
};
type EndEvent = { generation: number; failed: boolean };

const STORAGE_KEY = 'background-audio:tracks:v1';
const VOLUME_KEY = 'background-audio:volume:v1';

function loadTracks(): Track[] {
  try {
    const saved: unknown = JSON.parse(localStorage.getItem(STORAGE_KEY) || '[]');
    if (!Array.isArray(saved)) return [];
    return saved.filter((value): value is Track =>
      typeof value?.id === 'string' &&
      typeof value?.url === 'string' &&
      typeof value?.title === 'string' &&
      (value?.platform === 'youtube' || value?.platform === 'twitch'));
  } catch {
    return [];
  }
}

function sourceFromUrl(input: string): { url: string; platform: Platform; title: string } {
  let value: URL;
  try { value = new URL(input.trim()); } catch { throw new Error('Inserisci un link valido con https://'); }
  if (value.protocol !== 'https:') throw new Error('Usa un URL HTTPS');
  const host = value.hostname.toLowerCase();
  if (['youtube.com', 'www.youtube.com', 'm.youtube.com', 'music.youtube.com', 'youtu.be'].includes(host)) {
    const id = value.searchParams.get('v') || value.pathname.split('/').filter(Boolean).at(-1) || '';
    if (!id) throw new Error('Link YouTube incompleto');
    return { url: value.href, platform: 'youtube', title: `YouTube · ${id}` };
  }
  if (['twitch.tv', 'www.twitch.tv', 'm.twitch.tv'].includes(host)) {
    const channel = value.pathname.split('/').filter(Boolean).at(0);
    if (!channel) throw new Error('Aggiungi il nome del canale Twitch');
    return { url: value.href, platform: 'twitch', title: `Twitch · ${channel}` };
  }
  throw new Error('Per ora sono supportati solo link YouTube e Twitch');
}

function messageOf(error: unknown): string {
  return error instanceof Error ? error.message : String(error);
}

export default function App() {
  const [tracks, setTracks] = useState<Track[]>(loadTracks);
  const [input, setInput] = useState('');
  const [volume, setVolume] = useState(() => {
    const n = Number(localStorage.getItem(VOLUME_KEY) || '70');
    return Number.isFinite(n) ? Math.min(100, Math.max(0, n)) : 70;
  });
  const [currentId, setCurrentId] = useState<string | null>(null);
  const [isActive, setActive] = useState(false);
  const [paused, setPaused] = useState(false);
  const [busy, setBusy] = useState(false);
  const [error, setError] = useState('');
  const [dependencies, setDependencies] = useState<Dependencies | null>(null);
  const [showSettings, setShowSettings] = useState(false);
  const [showHelp, setShowHelp] = useState(false);
  const [status, setStatus] = useState('Pronto per riprodurre');
  const generationRef = useRef<number | null>(null);
  const currentIdRef = useRef<string | null>(null);
  const tracksRef = useRef(tracks);
  const busyRef = useRef(false);
  const playRef = useRef<(track: Track) => Promise<void>>(async () => {});

  useEffect(() => { tracksRef.current = tracks; localStorage.setItem(STORAGE_KEY, JSON.stringify(tracks)); }, [tracks]);
  useEffect(() => { currentIdRef.current = currentId; }, [currentId]);
  useEffect(() => { localStorage.setItem(VOLUME_KEY, String(volume)); }, [volume]);

  const refreshDependencies = async () => {
    try { setDependencies(await invoke<Dependencies>('check_dependencies')); }
    catch { setDependencies(null); }
  };

  const play = async (track: Track): Promise<void> => {
    if (busyRef.current) return;
    busyRef.current = true;
    setBusy(true);
    setError('');
    setStatus(track.platform === 'twitch' ? 'Connessione allo stream Twitch...' : 'Apertura audio YouTube...');
    try {
      const generation = await invoke<number>('start_playback', { url: track.url, volume });
      generationRef.current = generation;
      currentIdRef.current = track.id;
      setCurrentId(track.id);
      setPaused(false);
      setActive(true);
      setStatus('Riproduzione avviata');
    } catch (e) {
      setError(messageOf(e));
      setStatus('Impossibile riprodurre');
    } finally {
      busyRef.current = false;
      setBusy(false);
    }
  };
  playRef.current = play;

  useEffect(() => {
    let disposed = false;
    let unlisten: (() => void) | null = null;
    void listen<EndEvent>('playback-ended', event => {
      if (event.payload.generation !== generationRef.current) return;
      generationRef.current = null;
      setActive(false);
      setPaused(false);
      if (event.payload.failed) {
        setError('Il flusso è terminato con un errore. Controlla la connessione o aggiorna yt-dlp/Streamlink.');
        setStatus('Errore riproduzione');
        return;
      }
      const list = tracksRef.current;
      const index = list.findIndex(item => item.id === currentIdRef.current);
      const next = index >= 0 ? list[index + 1] : undefined;
      if (next) void playRef.current(next);
      else setStatus('Playlist terminata');
    }).then(stop => {
      if (disposed) stop(); else unlisten = stop;
    }).catch(() => {});

    void Promise.all([invoke<Snapshot>('get_playback_state'), invoke<Dependencies>('check_dependencies')])
      .then(([snapshot, deps]) => {
        if (disposed) return;
        setDependencies(deps);
        if (snapshot.active && snapshot.currentUrl) {
          const match = tracksRef.current.find(item => item.url === snapshot.currentUrl);
          currentIdRef.current = match?.id ?? null;
          setCurrentId(match?.id ?? null);
          generationRef.current = snapshot.generation;
          setPaused(snapshot.paused);
          setActive(true);
          setStatus(snapshot.paused ? 'In pausa' : 'Riproduzione attiva');
        }
      }).catch(() => {});
    return () => { disposed = true; unlisten?.(); };
  }, []);

  const addUrl = async () => {
    setError('');
    try {
      const source = sourceFromUrl(input);
      const existing = tracksRef.current.find(item => item.url === source.url);
      const track: Track = existing ?? { id: crypto.randomUUID(), ...source };
      if (!existing) setTracks(prev => [...prev, track]);
      setInput('');
      await play(track);
    } catch (e) {
      setError(messageOf(e));
    }
  };

  const stop = async () => {
    generationRef.current = null;
    try {
      await invoke('stop_playback');
      setActive(false);
      setPaused(false);
      setStatus('Riproduzione arrestata');
      setError('');
    } catch (e) { setError(messageOf(e)); }
  };

  const togglePause = async () => {
    if (!isActive || busy) return;
    try {
      await invoke('set_paused', { paused: !paused });
      setPaused(!paused);
      setStatus(paused ? 'Riproduzione attiva' : 'In pausa');
    } catch (e) { setError(messageOf(e)); }
  };

  const jump = async (step: number) => {
    if (busy) return;
    const index = tracks.findIndex(track => track.id === currentIdRef.current);
    const target = tracks[index + step];
    if (target) await play(target);
    else if (step > 0 && tracks.length && index === -1) await play(tracks[0]);
  };

  const remove = async (id: string) => {
    if (id === currentIdRef.current) {
      await stop();
      setCurrentId(null);
      currentIdRef.current = null;
    }
    setTracks(prev => prev.filter(item => item.id !== id));
  };

  const saveVolume = async (nextVolume: number) => {
    try { await invoke('set_volume', { volume: nextVolume }); }
    catch (e) { if (isActive) setError(messageOf(e)); }
  };

  const current = tracks.find(item => item.id === currentId);
  const at = tracks.findIndex(item => item.id === currentId);
  const hasPrev = at > 0 && !busy;
  const hasNext = ((at >= 0 && at < tracks.length - 1) || (at === -1 && tracks.length > 0)) && !busy;
  const missingDeps = dependencies && (!dependencies.mpv || !dependencies.ytDlp || !dependencies.streamlink);

  return (
    <main className="app">
      <header className="header">
        <div className="brand-logo"><AudioLines size={23} strokeWidth={2.2} /></div>
        <div className="brand-title">
          <strong>Background Audio</strong>
          <span>YOUR SOUND. NO VIDEO.</span>
        </div>
        <button className="icon-button settings-trigger" title="Impostazioni" onClick={() => setShowSettings(value => !value)}>
          {showSettings ? <X size={18} /> : <Settings2 size={18} />}
        </button>
      </header>

      {showSettings ? (
        <section className="settings-panel">
          <div className="section-title">Diagnostica motore <button className="tiny-button" onClick={() => void refreshDependencies()}><RotateCcw size={13}/> Aggiorna</button></div>
          <div className="deps">
            {([['MPV', 'mpv'], ['yt-dlp', 'ytDlp'], ['Streamlink', 'streamlink']] as const).map(([name, key]) => (
              <div className="dep" key={name}>
                <span>{name}</span>
                {dependencies?.[key] ? <span className="dep-ok"><Check size={14}/> Trovato</span> : <span className="dep-bad"><AlertCircle size={14}/> Non rilevato</span>}
              </div>
            ))}
          </div>
          <p className="footnote">Le dipendenze devono essere nel PATH di Windows. Se le hai appena installate, riavvia l'app.</p>
          <p className="footnote">Chiudendo la finestra il player resta nella tray. Per terminarlo: icona tray → Esci.</p>
        </section>
      ) : (
        <>
          <section className="hero">
            <div className={`sound-art ${isActive && !paused ? 'art-live' : ''}`}>
              <Headphones size={52} strokeWidth={1.4} />
              <div className="art-glow" />
            </div>
            <div className="status"><span className={`status-dot ${isActive && !paused ? 'dot-live' : ''}`} />{busy ? 'Connessione' : isActive ? paused ? 'In pausa' : 'Audio attivo' : 'In attesa'}</div>
            <h1>{current?.title ?? 'Ascolta senza video'}</h1>
            <p className="hero-description">{status}</p>
            <div className="controls">
              <button className="icon-button" aria-label="Precedente" onClick={() => void jump(-1)} disabled={!hasPrev}><SkipBack size={21} fill="currentColor"/></button>
              <button className="play-button" aria-label={isActive && !paused ? 'Pausa' : 'Riproduci'} disabled={busy || (!isActive && tracks.length === 0)} onClick={() => {
                if (isActive) void togglePause();
                else if (current) void play(current);
                else if (tracks[0]) void play(tracks[0]);
              }}>
                {busy ? <LoaderCircle className="spinning" size={25}/> : isActive && !paused ? <Pause fill="currentColor" size={26}/> : <Play fill="currentColor" size={25}/>}
              </button>
              <button className="icon-button" aria-label="Successivo" onClick={() => void jump(1)} disabled={!hasNext}><SkipForward size={21} fill="currentColor"/></button>
            </div>
            <div className="volume-row">
              <button className="icon-button volume-icon" aria-label="Mute" onClick={() => { setVolume(volume > 0 ? 0 : 70); void saveVolume(volume > 0 ? 0 : 70); }}>
                {volume === 0 ? <VolumeX size={18}/> : <Volume2 size={18}/>}
              </button>
              <input aria-label="Volume" type="range" min="0" max="100" step="1" value={volume} onChange={e => setVolume(Number(e.target.value))} onPointerUp={e => void saveVolume(Number((e.target as HTMLInputElement).value))} onKeyUp={e => void saveVolume(Number((e.target as HTMLInputElement).value))} style={{ '--volume': `${volume}%` } as CSSProperties} />
              <span className="volume-value">{volume}%</span>
              <button className="stop-button" aria-label="Stop" title="Stop" disabled={!isActive} onClick={() => void stop()}><Square size={15} fill="currentColor"/></button>
            </div>
          </section>

          <section className="url-section">
            <div className="url-header"><span>AGGIUNGI SORGENTE</span><div><Radio size={13}/> Twitch <span className="dot-divider">·</span> YouTube</div></div>
            <form className="url-form" onSubmit={event => { event.preventDefault(); void addUrl(); }}>
              <Link2 size={18} className="link-icon" />
              <input aria-label="URL YouTube o Twitch" value={input} onChange={e => setInput(e.target.value)} placeholder="Incolla un link YouTube o Twitch..." />
              <button type="submit" aria-label="Aggiungi e riproduci" disabled={!input.trim() || busy}><Plus size={19}/></button>
            </form>
            {error && <div className="error-message"><AlertCircle size={15}/><span>{error}</span><button aria-label="Chiudi errore" onClick={() => setError('')}><X size={13}/></button></div>}
            {missingDeps && <button className="warning" onClick={() => setShowSettings(true)}><AlertCircle size={15}/> Componenti non rilevati. Apri Diagnostica <ChevronDown size={14}/></button>}
          </section>

          <section className="queue-section">
            <div className="queue-head"><h2><ListMusic size={17}/> Playlist <span>{tracks.length}</span></h2>
              <button className="tiny-button" onClick={() => setShowHelp(value => !value)} title="Come funziona"><CircleHelp size={16}/></button>
            </div>
            {showHelp && <p className="help">Aggiungi i link uno alla volta. Puoi cliccare su qualsiasi elemento per riprodurlo. Al termine di un video YouTube parte il successivo.</p>}
            <div className="queue-list">
              {tracks.length === 0 ? (
                <div className="queue-empty"><ListMusic size={24}/><span>Nessuna sorgente aggiunta</span><small>I link appariranno qui</small></div>
              ) : tracks.map((item, index) => {
                const active = item.id === currentId;
                return <div className={`queue-item ${active ? 'queue-active' : ''}`} key={item.id}>
                  <button className="queue-main" onClick={() => void play(item)} title={item.url} disabled={busy}>
                    <span className="queue-number">{active && isActive && !paused ? <AudioLines size={16}/> : String(index + 1).padStart(2, '0')}</span>
                    <span className="queue-info"><strong>{item.title}</strong><span className="queue-url">{item.url}</span></span>
                    <span className={`platform-tag ${item.platform}`}>{item.platform === 'twitch' ? 'LIVE' : 'YT'}</span>
                  </button>
                  <button className="queue-delete" onClick={() => void remove(item.id)} title="Rimuovi"><Trash2 size={16}/></button>
                </div>;
              })}
            </div>
          </section>
          <footer><span><span className="footer-dot"/> ENGINE: MPV</span><span>AUDIO ONLY · v0.1</span></footer>
        </>
      )}
    </main>
  );
}

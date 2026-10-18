import { useEffect, useRef, useState } from 'react';
import type { CSSProperties } from 'react';
import { invoke } from '@tauri-apps/api/core';
import { listen } from '@tauri-apps/api/event';
import { getCurrentWindow } from '@tauri-apps/api/window';
import { isTauri } from '@tauri-apps/api/core';
import { ChatPanel, TwitchSettings, useTwitchChat } from './TwitchChat';
import { YoutubePanel, YoutubeSettings, useYoutubeChat } from './YoutubeChat';
import { useVideoView, VideoSurface, VideoToolbar } from './VideoPanel';
import type { PlaybackMode, Snapshot, EndEvent } from './player-types';
import {
  AlertCircle, AudioLines, Check, Headphones, Search, Minus, MessageCircle,
  ListMusic, LoaderCircle, Pause, Play, ArrowUpRight, RotateCcw,
  Settings2, SkipBack, SkipForward, Square, Trash2, Volume2, VolumeX,
  Video, X,
} from 'lucide-react';

type Platform = 'youtube' | 'twitch';
type Track = { id: string; url: string; title: string; platform: Platform };
type Dependencies = { mpv: boolean; ytDlp: boolean; streamlink: boolean };

const STORAGE_KEY = 'background-audio:tracks:v1';
const VOLUME_KEY = 'background-audio:volume:v1';
const MODE_KEY = 'background-audio:mode:v1';

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
  const trimmed = input.trim();
  if (/^@?[a-zA-Z0-9_]{1,25}$/.test(trimmed)) input = `https://www.twitch.tv/${trimmed.replace(/^@/, '')}`;
  else if (/^(www\.)?(twitch\.tv|youtube\.com|youtu\.be)\//i.test(trimmed)) input = `https://${trimmed}`;
  let value: URL;
  try { value = new URL(input.trim()); } catch { throw new Error('Inserisci un link valido con https://'); }
  if (value.protocol !== 'https:') throw new Error('Usa un URL HTTPS');
  if (value.username || value.password) throw new Error('Il link non può contenere credenziali');
  const host = value.hostname.toLowerCase();
  if (['youtube.com', 'www.youtube.com', 'm.youtube.com', 'music.youtube.com', 'youtu.be'].includes(host)) {
    const id = value.searchParams.get('v') || value.pathname.split('/').filter(Boolean).at(-1) || '';
    if (!id) throw new Error('Link YouTube incompleto');
    return { url: value.href, platform: 'youtube', title: `YouTube · ${id}` };
  }
  if (['twitch.tv', 'www.twitch.tv', 'm.twitch.tv'].includes(host)) {
    const channel = value.pathname.split('/').filter(Boolean).at(0);
    if (!channel || !/^[a-zA-Z0-9_]{1,25}$/.test(channel) || value.pathname.split('/').filter(Boolean).length !== 1 || ['videos', 'directory', 'downloads', 'settings', 'search'].includes(channel.toLowerCase())) throw new Error('Inserisci un canale Twitch live, non un video o una pagina Twitch');
    return { url: `https://www.twitch.tv/${channel.toLowerCase()}`, platform: 'twitch', title: channel };
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
  const [mode, setMode] = useState<PlaybackMode>(() => localStorage.getItem(MODE_KEY) === 'video_audio' ? 'video_audio' : 'audio_only');
  const [busy, setBusy] = useState(false);
  const [error, setError] = useState('');
  const videoView = useVideoView(setError);
  const [dependencies, setDependencies] = useState<Dependencies | null>(null);
  const [tab, setTab] = useState<'player' | 'library' | 'settings'>('player');
  const [status, setStatus] = useState('Pronto per riprodurre');
  const generationRef = useRef<number | null>(null);
  const currentIdRef = useRef<string | null>(null);
  const tracksRef = useRef(tracks);
  const busyRef = useRef(false);
  const modeRef = useRef(mode);
  const activeTrackRef = useRef<Track | null>(null);
  const pendingEndRef = useRef<EndEvent | null>(null);
  const pendingStopRef = useRef<Snapshot | null>(null);
  const handleEndedRef = useRef<(event: EndEvent) => void>(() => {});
  const playRef = useRef<(track: Track) => Promise<void>>(async () => {});
  const controlActionRef = useRef<(action: string) => void>(() => {});

  useEffect(() => { tracksRef.current = tracks; localStorage.setItem(STORAGE_KEY, JSON.stringify(tracks)); }, [tracks]);
  useEffect(() => { currentIdRef.current = currentId; }, [currentId]);
  useEffect(() => { localStorage.setItem(VOLUME_KEY, String(volume)); }, [volume]);
  useEffect(() => { localStorage.setItem(MODE_KEY, mode); }, [mode]);
  useEffect(() => { if (videoView.fullscreen && !videoView.detached) setTab('player'); }, [videoView.fullscreen, videoView.detached]);
  useEffect(() => {
    if (!isTauri()) return;
    let disposed = false;
    const cleanup: (() => void)[] = [];
    const register = async () => {
      for (const subscription of [
        listen<string>('player-control-request', event => controlActionRef.current(event.payload)),
        listen<string>('player-control-error', event => setError(event.payload)),
        listen('open-twitch-settings', () => setTab('settings')),
      ]) {
        const stop = await subscription;
        if (disposed) stop(); else cleanup.push(stop);
      }
    };
    void register().catch(e => { if (!disposed) setError(messageOf(e)); });
    return () => { disposed = true; cleanup.forEach(stop => stop()); };
  }, []);

  const refreshDependencies = async () => {
    try { setDependencies(await invoke<Dependencies>('check_dependencies')); }
    catch { setDependencies(null); }
  };

  const play = async (track: Track, nextMode = modeRef.current): Promise<void> => {
    if (busyRef.current) return;
    busyRef.current = true;
    setBusy(true);
    setError('');
    setStatus(track.platform === 'twitch' ? 'Connessione allo stream Twitch...' : nextMode === 'video_audio' ? 'Apertura video YouTube...' : 'Apertura audio YouTube...');
    try {
      const generation = await invoke<number>('start_playback', { url: track.url, volume, mode: nextMode });
      generationRef.current = generation;
      currentIdRef.current = track.id;
      activeTrackRef.current = track;
      modeRef.current = nextMode;
      setMode(nextMode);
      setCurrentId(track.id);
      setPaused(false);
      setActive(true);
      setTab('player');
      setStatus(nextMode === 'video_audio' ? 'Video nel widget' : 'Riproduzione avviata');
    } catch (e) {
      setError(messageOf(e));
      setStatus('Impossibile riprodurre');
      // A native host error can occur after MPV replaced the old process.
      try {
        const snapshot = await invoke<Snapshot>('get_playback_state');
        if (!snapshot.active) { generationRef.current = null; setActive(false); setPaused(false); }
      } catch { /* Keep the original concrete playback error. */ }
    } finally {
      busyRef.current = false;
      setBusy(false);
      const stopped = pendingStopRef.current;
      pendingStopRef.current = null;
      if (stopped && stopped.generation > (generationRef.current ?? 0)) {
        generationRef.current = null;
        setActive(false);
        setPaused(false);
        setStatus('Riproduzione arrestata');
      }
      // A process can finish while Rust is resolving or starting the next stream.
      const pending = pendingEndRef.current;
      pendingEndRef.current = null;
      if (pending) handleEndedRef.current(pending);
    }
  };
  playRef.current = play;

  handleEndedRef.current = payload => {
    if (payload.generation !== generationRef.current) return;
    generationRef.current = null;
    setActive(false);
    setPaused(false);
    if (payload.failed) {
      setError('Il flusso è terminato con un errore. Controlla la connessione o aggiorna yt-dlp/Streamlink.');
      setStatus('Errore riproduzione');
      return;
    }
    const list = tracksRef.current;
    const index = list.findIndex(item => item.id === currentIdRef.current);
    const next = index >= 0 ? list[index + 1] : undefined;
    if (next) void playRef.current(next);
    else setStatus('Playlist terminata');
  };

  const changeMode = async (nextMode: PlaybackMode) => {
    if (busyRef.current || nextMode === modeRef.current) return;
    if (isActive && activeTrackRef.current) {
      await play(activeTrackRef.current, nextMode);
    } else {
      busyRef.current = true;
      setBusy(true);
      try {
        // EOF may leave the video view fullscreen or detached, even while idle.
        if (nextMode === 'audio_only') {
          await invoke('set_video_fullscreen', { fullscreen: false });
          await invoke('set_video_detached', { detached: false });
        }
        modeRef.current = nextMode;
        setMode(nextMode);
        setStatus(nextMode === 'video_audio' ? 'Video pronto nel widget' : 'Pronto per ascoltare senza video');
      } catch (e) { setError(messageOf(e)); }
      finally { busyRef.current = false; setBusy(false); }
    }
  };

  useEffect(() => {
    let disposed = false;
    let unlisten: (() => void) | null = null;
    let unlistenState: (() => void) | null = null;
    void listen<Snapshot>('playback-state-changed', event => {
      if (!event.payload.active && busyRef.current) { pendingStopRef.current = event.payload; return; }
      if (!event.payload.active && !busyRef.current && event.payload.generation >= (generationRef.current ?? 0)) {
        generationRef.current = null;
        setActive(false);
        setPaused(false);
        setStatus('Riproduzione arrestata');
        return;
      }
      if (event.payload.generation !== generationRef.current) return;
      setPaused(event.payload.paused);
      setVolume(event.payload.volume);
      if (!event.payload.active) { generationRef.current = null; setActive(false); }
    }).then(stop => { if (disposed) stop(); else unlistenState = stop; }).catch(() => {});
    void listen<EndEvent>('playback-ended', event => {
      if (busyRef.current) pendingEndRef.current = event.payload;
      else handleEndedRef.current(event.payload);
    }).then(stop => {
      if (disposed) stop(); else unlisten = stop;
    }).catch(() => {});

    void Promise.all([invoke<Snapshot>('get_playback_state'), invoke<Dependencies>('check_dependencies')])
      .then(([snapshot, deps]) => {
        if (disposed) return;
        setDependencies(deps);
        if (snapshot.active && snapshot.currentUrl && !busyRef.current && generationRef.current === null) {
          const match = tracksRef.current.find(item => item.url === snapshot.currentUrl);
          activeTrackRef.current = match ?? { id: crypto.randomUUID(), ...sourceFromUrl(snapshot.currentUrl) };
          modeRef.current = snapshot.mode;
          setMode(snapshot.mode);
          setVolume(snapshot.volume);
          currentIdRef.current = match?.id ?? null;
          setCurrentId(match?.id ?? null);
          generationRef.current = snapshot.generation;
          setPaused(snapshot.paused);
          setActive(true);
          setStatus(snapshot.paused ? 'In pausa' : 'Riproduzione attiva');
        }
      }).catch(() => {});
    return () => { disposed = true; unlisten?.(); unlistenState?.(); };
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
  controlActionRef.current = action => {
    if (action === 'previous') void jump(-1);
    else if (action === 'next') void jump(1);
    else if (action === 'audio') void changeMode('audio_only');
  };

  const current = tracks.find(item => item.id === currentId) ?? activeTrackRef.current;
  const channel = isActive && current?.platform === 'twitch' ? new URL(current.url).pathname.split('/').filter(Boolean)[0] ?? null : null;
  const chat = useTwitchChat(channel);
  const youtubeUrl = isActive && current?.platform === 'youtube' ? current.url : null;
  const youtubeChat = useYoutubeChat(youtubeUrl);
  const at = tracks.findIndex(item => item.id === currentId);
  const hasPrev = at > 0 && !busy;
  const hasNext = ((at >= 0 && at < tracks.length - 1) || (at === -1 && tracks.length > 0)) && !busy;
  const missingDeps = dependencies && (!dependencies.mpv || !dependencies.ytDlp || !dependencies.streamlink);

  const windowAction = async (action: 'minimize' | 'close') => {
    if (!isTauri()) return;
    try { await getCurrentWindow()[action](); } catch (e) { setError(messageOf(e)); }
  };
  const tabs = [
    { id: 'player', label: 'Player', icon: AudioLines },
    { id: 'library', label: 'Libreria', icon: ListMusic },
    { id: 'settings', label: 'Impostazioni', icon: Settings2 },
  ] as const;

  const playbackControls = (
    <section className="player-controls" aria-label="Controlli di riproduzione">
        <div className="now-playing"><div className="track-mark">{current?.platform === 'youtube' ? <Video size={18}/> : <AudioLines size={18}/>}</div><div className="track-details"><strong>{current?.title.replace(/^Twitch · /, '') ?? 'Nessuna sorgente'}</strong><span role="status">{busy ? 'Connessione…' : isActive && paused ? 'In pausa' : status}</span></div>{isActive && current?.platform === 'twitch' && <span className="live-pill">LIVE</span>}</div>
        <div className="controls-row">
          <div className="transport">
            <button className="icon-button" title="Precedente" aria-label="Precedente" onClick={() => void jump(-1)} disabled={!hasPrev}><SkipBack size={18}/></button>
            <button className="play-button" title={isActive && !paused ? 'Pausa' : 'Riproduci'} aria-label={isActive && !paused ? 'Pausa' : 'Riproduci'} disabled={busy || (!isActive && !current && !tracks.length)} onClick={() => {
              if (isActive) void togglePause(); else if (current) void play(current); else if (tracks[0]) void play(tracks[0]);
            }}>{busy ? <LoaderCircle className="spinning" size={20}/> : isActive && !paused ? <Pause size={20} fill="currentColor"/> : <Play size={20} fill="currentColor"/>}</button>
            <button className="icon-button" title="Successivo" aria-label="Successivo" onClick={() => void jump(1)} disabled={!hasNext}><SkipForward size={18}/></button>
            <button className="icon-button" title="Stop" aria-label="Stop" disabled={!isActive || busy} onClick={() => void stop()}><Square size={14}/></button>
          </div>
          <div className="mode-switch" role="group" aria-label="Modalità di riproduzione">
            <button title="Solo audio · il cambio riavvia la sorgente" aria-label="Solo audio" aria-pressed={mode === 'audio_only'} disabled={busy} onClick={() => void changeMode('audio_only')}><Headphones size={16}/></button>
            <button title="Video e audio · il cambio riavvia la sorgente" aria-label="Video e audio" aria-pressed={mode === 'video_audio'} disabled={busy} onClick={() => void changeMode('video_audio')}><Video size={16}/></button>
          </div>
          {mode === 'video_audio' && <VideoToolbar view={videoView} disabled={busy} onError={setError}/>}
        </div>
        <div className="volume-row"><button className="icon-button" title={volume ? 'Disattiva audio' : 'Riattiva audio'} aria-label={volume ? 'Disattiva audio' : 'Riattiva audio'} onClick={() => { setVolume(volume > 0 ? 0 : 70); void saveVolume(volume > 0 ? 0 : 70); }}>{volume ? <Volume2 size={16}/> : <VolumeX size={16}/>}</button>
          <input aria-label="Volume" type="range" min="0" max="100" step="1" value={volume} onChange={e => setVolume(Number(e.target.value))} onPointerUp={e => void saveVolume(Number(e.currentTarget.value))} onKeyUp={e => void saveVolume(Number(e.currentTarget.value))} style={{ '--volume': `${volume}%` } as CSSProperties}/><span>{volume}%</span>
        </div>
      </section>
  );

  return (
    <main className={`app ${videoView.fullscreen && !videoView.detached ? 'video-fullscreen' : ''}`}>
      <header className="topbar">
        <div className="drag-handle" data-tauri-drag-region title="Trascina la finestra"><span/><span/></div>
        <form className="search-form" onSubmit={event => { event.preventDefault(); void addUrl(); }}>
          <Search size={16} aria-hidden="true"/>
          <input aria-label="Canale Twitch o link YouTube" value={input} onChange={e => setInput(e.target.value)} placeholder="Canale Twitch o link…" spellCheck={false}/>
          <button type="submit" title="Apri e riproduci" aria-label="Apri e riproduci" disabled={!input.trim() || busy}><ArrowUpRight size={17}/></button>
        </form>
        <div className="window-actions">
          <button className="icon-button" title="Riduci a icona" aria-label="Riduci a icona" onClick={() => void windowAction('minimize')}><Minus size={15}/></button>
          <button className="icon-button" title="Nascondi nella tray" aria-label="Nascondi nella tray" onClick={() => void windowAction('close')}><X size={15}/></button>
        </div>
      </header>
      <nav className="navigation" aria-label="Viste">
        {tabs.map(({ id, label, icon: Icon }) => <button key={id} aria-current={tab === id ? 'page' : undefined} onClick={() => setTab(id)}><Icon size={16}/><span>{label}</span>{id === 'library' && tracks.length > 0 && <small>{tracks.length}</small>}</button>)}
      </nav>
      {error && <div className="error-message" role="alert"><AlertCircle size={15}/><span>{error}</span><button aria-label="Chiudi errore" onClick={() => setError('')}><X size={14}/></button></div>}
      {missingDeps && <button className="warning" onClick={() => setTab('settings')}><AlertCircle size={14}/> Componenti mancanti · Apri impostazioni <ArrowUpRight size={14}/></button>}
      <div className="content">
        <section className={`player-page ${(channel && !chat.state.detached) || (youtubeUrl && !youtubeChat.state.detached) ? 'with-chat' : ''}`} hidden={tab !== 'player'} aria-label="Player">
          <div className={`media-player ${mode === 'video_audio' ? 'media-video' : 'media-audio'}`}>
          <div className={`player-stage ${mode === 'video_audio' ? 'player-video' : ''}`}>
          {mode === 'video_audio' ? <VideoSurface view={videoView} onError={setError}/> :
            <div className={`audio-visual ${isActive && !paused ? 'playing' : ''}`} aria-hidden="true">
              <div className="audio-halo"/>
              <div className="waveform">{[13, 24, 37, 18, 49, 70, 44, 29, 56, 82, 61, 35, 50, 72, 43, 24, 54, 36, 19, 29, 12].map((height, i) => <i key={i} style={{ '--bar-height': `${height}px`, '--delay': `${i * -0.13}s` } as CSSProperties}/>)}</div>
              <span className="visual-label">{isActive ? paused ? 'IN PAUSA' : 'SOLO AUDIO' : 'PRONTO AD ASCOLTARE'}</span>
            </div>}
          {!current && <div className="stage-caption">Incolla un link o scrivi un canale Twitch.</div>}
          </div>
          {(mode !== 'video_audio' || !videoView.enabled || videoView.detached) && playbackControls}
          </div>
          {channel && (chat.state.detached ? <button className="chat-window-link text-button" onClick={() => void chat.command('chat_set_detached', { detached: false })}><MessageCircle size={14}/> Chat in finestra · Reinserisci</button> : <ChatPanel key={channel} channel={channel} chat={chat} onSettings={() => setTab('settings')}/>)}
          {youtubeUrl && (youtubeChat.state.detached ? <button className="chat-window-link text-button" onClick={() => void youtubeChat.command('youtube_set_detached', { detached: false })}><MessageCircle size={14}/> YouTube in finestra · Reinserisci</button> : <YoutubePanel key={youtubeUrl} chat={youtubeChat} onSettings={() => setTab('settings')}/>)}
        </section>
        {tab === 'library' && <section className="library-panel">
          <div className="panel-heading"><h1>Libreria <span className="count">{tracks.length}</span></h1></div>
          <p className="panel-caption">Le tue sorgenti, pronte da riprodurre.</p>
          <div className="queue-list">
            {!tracks.length ? <div className="empty-state"><ListMusic size={28}/><p>La tua prossima sorgente va qui.</p><small>Aggiungi un canale o un link dalla barra in alto.</small></div> : tracks.map(item => <div className={`queue-item ${item.id === currentId ? 'queue-active' : ''}`} key={item.id}>
              <button className="queue-main" onClick={() => void play(item)} title={item.url} disabled={busy}>
                <span className="source-icon">{item.platform === 'twitch' ? <AudioLines size={19}/> : <Video size={19}/>}</span>
                <span className="queue-info"><strong>{item.title.replace(/^Twitch · /, '')}</strong><span>{item.platform === 'twitch' ? 'Twitch · diretta' : 'YouTube'}</span></span>
                {item.id === currentId && isActive ? <span className="status-dot dot-live"/> : <Play size={14}/>}
              </button>
              <button className="icon-button queue-delete" disabled={busy} onClick={() => void remove(item.id)} aria-label={`Rimuovi ${item.title}`} title="Rimuovi dalla libreria"><Trash2 size={15}/></button>
            </div>)}
          </div>
        </section>}
        {tab === 'settings' && <section className="settings-panel">
          <div className="panel-heading"><h1>Impostazioni</h1></div>
          <TwitchSettings chat={chat}/>
          <YoutubeSettings chat={youtubeChat}/>
          <section className="settings-group">
            <h2>Motore di riproduzione <button className="icon-button" title="Aggiorna diagnostica" aria-label="Aggiorna diagnostica" onClick={() => void refreshDependencies()}><RotateCcw size={14}/></button></h2>
            <div className="deps">{([['MPV', 'mpv'], ['yt-dlp', 'ytDlp'], ['Streamlink', 'streamlink']] as const).map(([name, key]) => <div className="dep" key={name}><span>{name}</span><span className={dependencies?.[key] ? 'dep-ok' : 'dep-bad'}>{dependencies?.[key] ? <><Check size={13}/> Disponibile</> : <><AlertCircle size={13}/> Non rilevato</>}</span></div>)}</div>
            <p className="footnote">Installa i componenti nel PATH di Windows e riavvia l’app.</p>
          </section>
          <details className="settings-group"><summary>Riproduzione e finestra</summary><p className="footnote">Cambiare Solo audio / Video riapre la sorgente: YouTube riparte dall’inizio e Twitch torna alla diretta. Solo audio richiede audio_only; se manca, la riproduzione si ferma.</p><p className="footnote">Al termine normale parte la sorgente successiva; un errore ferma la playlist. Puoi staccare il video o espanderlo senza riconnetterti.</p><p className="footnote">Chiudere la finestra la nasconde nella tray. Per terminare l’app: icona tray → Esci.</p></details>
        </section>}
      </div>
      {tab !== 'player' && <div className="background-controls">{playbackControls}</div>}
    </main>
  );
}

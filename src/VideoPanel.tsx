import { useEffect, useLayoutEffect, useRef, useState } from 'react';
import { invoke, isTauri } from '@tauri-apps/api/core';
import { listen } from '@tauri-apps/api/event';
import { getCurrentWindow } from '@tauri-apps/api/window';
import { ArrowDownLeft, Maximize, Minimize, Pause, Play, Square, ExternalLink, Video } from 'lucide-react';
import type { EndEvent, Snapshot } from './player-types';

type VideoView = { detached: boolean; fullscreen: boolean; enabled: boolean };
type ErrorHandler = (message: string) => void;

export function useVideoView(onError: ErrorHandler) {
  const [view, setView] = useState<VideoView>({ detached: false, fullscreen: false, enabled: false });
  useEffect(() => {
    if (!isTauri()) return;
    let disposed = false;
    const cleanup: (() => void)[] = [];
    const subscribe = async () => {
      const stopView = await listen<VideoView>('video-view-changed', event => { if (!disposed) setView(event.payload); });
      if (disposed) { stopView(); return; }
      cleanup.push(stopView);
      const stopError = await listen<string>('video-view-error', event => { if (!disposed) onError(event.payload); });
      if (disposed) { stopError(); return; }
      cleanup.push(stopError);
      const initial = await invoke<VideoView>('get_video_view');
      if (!disposed) setView(initial);
    };
    void subscribe().catch(error => { if (!disposed) onError(String(error)); });
    return () => { disposed = true; cleanup.forEach(stop => stop()); };
  }, [onError]);
  return view;
}

export function VideoSurface({ view, onError }: { view: VideoView; onError: ErrorHandler }) {
  const element = useRef<HTMLDivElement>(null);
  const detachedWindow = isTauri() && getCurrentWindow().label === 'video';
  const ownsVideo = detachedWindow === view.detached;

  useLayoutEffect(() => {
    if (!isTauri() || !element.current) return;
    const target = element.current;
    let frame = 0;
    let disposed = false;
    const report = (visible: boolean) => {
      const rect = target.getBoundingClientRect();
      void invoke('update_video_bounds', { bounds: {
        x: Math.max(0, rect.x), y: Math.max(0, rect.y), width: rect.width, height: rect.height,
        scale: window.devicePixelRatio, visible,
        revision: performance.timeOrigin + performance.now(),
      } }).catch(error => { if (!disposed) onError(String(error)); });
    };
    const measure = () => {
      cancelAnimationFrame(frame);
      frame = requestAnimationFrame(() => report(view.enabled && ownsVideo));
    };
    const observer = new ResizeObserver(measure);
    observer.observe(target);
    window.addEventListener('resize', measure);
    let stopScale: (() => void) | null = null;
    void getCurrentWindow().onScaleChanged(measure).then(stop => { if (disposed) stop(); else stopScale = stop; }).catch(() => {});
    measure();
    return () => {
      disposed = true;
      observer.disconnect();
      window.removeEventListener('resize', measure);
      cancelAnimationFrame(frame);
      stopScale?.();
      report(false);
    };
  }, [view.enabled, view.detached, view.fullscreen, ownsVideo, onError]);

  return <div ref={element} className="video-surface" aria-label="Area video">
    <Video size={30}/>
    <span>{!ownsVideo ? 'Video nella finestra staccata' : view.enabled ? 'Apertura video…' : 'Il video apparirà qui'}</span>
  </div>;
}

export function VideoToolbar({ view, disabled = false, onError }: { view: VideoView; disabled?: boolean; onError: ErrorHandler }) {
  const [working, setWorking] = useState(false);
  const change = async (command: string, args: Record<string, boolean>) => {
    if (working) return;
    setWorking(true);
    try { await invoke(command, args); } catch (error) { onError(String(error)); }
    finally { setWorking(false); }
  };
  useEffect(() => {
    const escape = (event: KeyboardEvent) => {
      if (event.key === 'Escape' && view.fullscreen) void change('set_video_fullscreen', { fullscreen: false });
    };
    window.addEventListener('keydown', escape);
    return () => window.removeEventListener('keydown', escape);
  }, [view.fullscreen, working]);

  return <div className="video-toolbar">
    <button className="icon-button" disabled={disabled || working || (!view.enabled && !view.detached)}
      title={view.detached ? 'Reinserisci nel widget' : 'Stacca il player'} aria-label={view.detached ? 'Reinserisci nel widget' : 'Stacca il player'}
      onClick={() => void change('set_video_detached', { detached: !view.detached })}>
      {view.detached ? <ArrowDownLeft size={17}/> : <ExternalLink size={17}/>}
    </button>
    <button className="icon-button" disabled={disabled || working || (!view.enabled && !view.fullscreen)}
      title={view.fullscreen ? 'Esci da schermo intero' : 'Schermo intero'} aria-label={view.fullscreen ? 'Esci da schermo intero' : 'Schermo intero'}
      onClick={() => void change('set_video_fullscreen', { fullscreen: !view.fullscreen })}>
      {view.fullscreen ? <Minimize size={17}/> : <Maximize size={17}/>}
    </button>
  </div>;
}

export function DetachedPlayer() {
  const [error, setError] = useState('');
  const view = useVideoView(setError);
  const [snapshot, setSnapshot] = useState<Snapshot | null>(null);
  const generation = useRef<number | null>(null);
  useEffect(() => {
    let disposed = false;
    const cleanup: (() => void)[] = [];
    const subscribe = async () => {
      const stopState = await listen<Snapshot>('playback-state-changed', event => {
        if (!disposed) { generation.current = event.payload.generation; setSnapshot(event.payload); if (event.payload.active) setError(''); }
      });
      if (disposed) { stopState(); return; }
      cleanup.push(stopState);
      const stopEnd = await listen<EndEvent>('playback-ended', event => {
        if (disposed || event.payload.generation !== generation.current) return;
        setSnapshot(previous => previous?.generation === event.payload.generation ? { ...previous, active: false, paused: false } : previous);
        if (event.payload.failed) setError('Il flusso è terminato con un errore. Controlla connessione e dipendenze.');
      });
      if (disposed) { stopEnd(); return; }
      cleanup.push(stopEnd);
      const initial = await invoke<Snapshot>('get_playback_state');
      if (!disposed) { generation.current = initial.generation; setSnapshot(initial); }
    };
    void subscribe().catch(e => { if (!disposed) setError(String(e)); });
    return () => { disposed = true; cleanup.forEach(stop => stop()); };
  }, []);
  const control = async (command: string, args?: Record<string, unknown>) => {
    setError('');
    try { await invoke(command, args); } catch (e) { setError(String(e)); }
  };
  return <main className="detached-player">
    <VideoSurface view={view} onError={setError}/>
    <div className="detached-controls">
      <button className="icon-button" disabled={!snapshot?.active} aria-label={snapshot?.paused ? 'Riprendi' : 'Pausa'}
        onClick={() => void control('set_paused', { paused: !snapshot?.paused })}>
        {snapshot?.paused ? <Play size={18}/> : <Pause size={18}/>}
      </button>
      <button className="icon-button" disabled={!snapshot?.active} aria-label="Stop" onClick={() => void control('stop_playback')}><Square size={15}/></button>
      <input aria-label="Volume" type="range" min="0" max="100" value={snapshot?.volume ?? 70}
        onChange={event => { const volume = Number(event.target.value); setSnapshot(previous => previous ? { ...previous, volume } : previous); }}
        onPointerUp={event => void control('set_volume', { volume: Number(event.currentTarget.value) })}
        onKeyUp={event => void control('set_volume', { volume: Number(event.currentTarget.value) })}/>
      <VideoToolbar view={view} onError={setError}/>
    </div>
    {error && <div className="detached-error" role="alert">{error}</div>}
  </main>;
}

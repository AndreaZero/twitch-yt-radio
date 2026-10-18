import { useEffect, useRef, useState } from 'react';
import { invoke } from '@tauri-apps/api/core';
import { emitTo, listen } from '@tauri-apps/api/event';
import { Headphones, Pause, Play, SkipBack, SkipForward, Square, Volume2, VolumeX } from 'lucide-react';
import { useVideoView, VideoToolbar } from './VideoPanel';
import type { EndEvent, Snapshot } from './player-types';

export function VideoControls() {
  const [snapshot, setSnapshot] = useState<Snapshot | null>(null);
  const [error, setError] = useState('');
  const [working, setWorking] = useState(false);
  const busy = useRef(false);
  const version = useRef(0);
  const view = useVideoView(setError);
  useEffect(() => {
    let disposed = false;
    const cleanup: (() => void)[] = [];
    const subscribe = async () => {
      const stopState = await listen<Snapshot>('playback-state-changed', event => {
        if (!disposed) { version.current++; setSnapshot(event.payload); setError(''); }
      });
      if (disposed) { stopState(); return; } cleanup.push(stopState);
      const stopEnd = await listen<EndEvent>('playback-ended', event => {
        if (!disposed) {
          version.current++;
          setSnapshot(previous => previous?.generation === event.payload.generation ? { ...previous, active: false, paused: false } : previous);
        }
      });
      if (disposed) { stopEnd(); return; } cleanup.push(stopEnd);
      const before = version.current;
      const initial = await invoke<Snapshot>('get_playback_state');
      if (!disposed && before === version.current) setSnapshot(initial);
    };
    void subscribe().catch(e => { if (!disposed) setError(String(e)); });
    return () => { disposed = true; cleanup.forEach(stop => stop()); };
  }, []);
  const control = async (name: string, args?: Record<string, unknown>) => {
    if (busy.current) return;
    busy.current = true; setWorking(true); setError('');
    try { await invoke(name, args); }
    catch (e) { setError(String(e)); void emitTo('main', 'player-control-error', String(e)); }
    finally { busy.current = false; setWorking(false); }
  };
  const request = (action: 'previous' | 'next' | 'audio') => {
    void emitTo('main', 'player-control-request', action).catch(e => setError(String(e)));
  };
  const volume = snapshot?.volume ?? 70;
  const pin = (interacting: boolean) => { void invoke('set_video_overlay_interacting', { interacting }).catch(e => setError(String(e))); };
  return <main className="video-overlay" aria-label="Controlli video in sovraimpressione"
    onPointerDownCapture={() => pin(true)} onPointerUpCapture={() => pin(false)} onPointerCancelCapture={() => pin(false)}>
    <div className="overlay-controls">
      <button className="icon-button" title="Precedente" aria-label="Precedente" disabled={working || !snapshot?.active} onClick={() => request('previous')}><SkipBack size={17}/></button>
      <button className="icon-button overlay-play" aria-label={snapshot?.paused ? 'Riprendi' : 'Pausa'} disabled={working || !snapshot?.active} onClick={() => void control('set_paused', { paused: !snapshot?.paused })}>{snapshot?.paused ? <Play size={19}/> : <Pause size={19}/>}</button>
      <button className="icon-button" title="Successivo" aria-label="Successivo" disabled={working || !snapshot?.active} onClick={() => request('next')}><SkipForward size={17}/></button>
      <button className="icon-button" title="Stop" aria-label="Stop" disabled={working || !snapshot?.active} onClick={() => void control('stop_playback')}><Square size={14}/></button>
      <button className="icon-button" title={volume ? 'Disattiva audio' : 'Riattiva audio'} aria-label={volume ? 'Disattiva audio' : 'Riattiva audio'} disabled={working} onClick={() => void control('set_volume', { volume: volume ? 0 : 70 })}>{volume ? <Volume2 size={16}/> : <VolumeX size={16}/>}</button>
      <input aria-label="Volume video" type="range" min="0" max="100" value={volume} disabled={working}
        onPointerDown={event => event.currentTarget.setPointerCapture(event.pointerId)}
        onChange={event => { const next = Number(event.currentTarget.value); setSnapshot(previous => previous ? { ...previous, volume: next } : previous); }}
        onPointerUp={event => void control('set_volume', { volume: Number(event.currentTarget.value) })}
        onKeyUp={event => void control('set_volume', { volume: Number(event.currentTarget.value) })}/>
      <button className="icon-button" title="Solo audio · riavvia la sorgente" aria-label="Passa a solo audio" disabled={working || !snapshot?.active} onClick={() => request('audio')}><Headphones size={16}/></button>
      <VideoToolbar view={view} disabled={working} onError={setError}/>
    </div>
    {error && <div className="overlay-error" role="alert" title={error}>{error}</div>}
  </main>;
}

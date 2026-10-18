import { useCallback, useEffect, useRef, useState } from 'react';
import { invoke, isTauri } from '@tauri-apps/api/core';
import { listen } from '@tauri-apps/api/event';
import { AlertCircle, Check, Download, ExternalLink, LoaderCircle, RotateCcw } from 'lucide-react';

type Tool = 'mpv' | 'yt-dlp' | 'streamlink' | 'ffmpeg' | 'deno';
type ToolStatus = { name: Tool; available: boolean; path: string | null; configuredPath: string | null; error: string; package: string };
type Progress = { version: number; running: boolean; current: string; message: string; completed: string[]; errors: string[] };
type Report = { tools: ToolStatus[]; winget: boolean; progress: Progress; configurationError: string };
const descriptions: Record<Tool, string> = { mpv: 'Riproduzione audio e video', 'yt-dlp': 'Sorgenti YouTube', streamlink: 'Dirette Twitch', ffmpeg: 'FFmpeg e FFprobe · supporto multimediale', deno: 'Runtime JavaScript per YouTube' };

export function useSetup() {
  const [report, setReport] = useState<Report | null>(null);
  const [progress, setProgress] = useState<Progress | null>(null);
  const [loading, setLoading] = useState(false);
  const [actionBusy, setActionBusy] = useState(false);
  const [error, setError] = useState('');
  const revision = useRef(0);
  const refresh = useCallback(async () => {
    if (!isTauri()) return;
    const ticket = ++revision.current;
    setLoading(true); setError('');
    try {
      const next = await invoke<Report>('setup_report');
      if (ticket !== revision.current) return;
      setReport(next);
      setProgress(previous => previous && previous.version > next.progress.version ? previous : next.progress);
    } catch (e) { if (ticket === revision.current) setError(String(e)); }
    finally { if (ticket === revision.current) setLoading(false); }
  }, []);
  useEffect(() => {
    if (!isTauri()) return;
    let disposed = false;
    let cleanup: (() => void) | undefined;
    void listen<Progress>('setup-progress', event => {
      if (disposed) return;
      setProgress(previous => previous && previous.version > event.payload.version ? previous : event.payload);
      if (!event.payload.running) void refresh();
    }).then(stop => { if (disposed) stop(); else { cleanup = stop; void refresh(); } }).catch(e => { if (!disposed) setError(String(e)); });
    return () => { disposed = true; cleanup?.(); revision.current++; };
  }, [refresh]);
  const command = async (name: string, args?: Record<string, unknown>) => {
    if (!isTauri()) return;
    setActionBusy(true); setError('');
    try {
      await invoke(name, args);
      if (name === 'setup_set_path') await refresh();
    } catch (e) { setError(String(e)); }
    finally { setActionBusy(false); }
  };
  return { report, progress, loading, error, refresh, command, busy: actionBusy || !!progress?.running };
}
export type SetupController = ReturnType<typeof useSetup>;

export function SetupWizard({ setup, onDone }: { setup: SetupController; onDone: () => void }) {
  const missing = setup.report?.tools.filter(tool => !tool.available) ?? [];
  return <section className="settings-group setup-wizard" aria-label="Configurazione guidata componenti">
    <div className="panel-heading"><h1>Prepariamo il player</h1><button className="icon-button" aria-label="Rileva di nuovo" title="Rileva di nuovo" disabled={setup.loading || setup.busy} onClick={() => void setup.refresh()}><RotateCcw size={15}/></button></div>
    <p className="footnote">Stessa guida per installer ed EXE. I componenti già disponibili vengono riutilizzati; non servono Node, Python o Rust per usare l’app compilata.</p>
    {!setup.report && <p className="footnote" role="status">{setup.loading ? 'Rilevamento componenti…' : 'Premi Rileva di nuovo per iniziare.'}</p>}
    {setup.report && <>
      {!setup.report.winget && <div className="setup-notice"><p>Manca WinGet. Installa o aggiorna <strong>App Installer</strong> di Microsoft, poi premi Rileva di nuovo.</p><button className="text-button" disabled={setup.busy} onClick={() => void setup.command('setup_open_download', { tool: null })}><ExternalLink size={13}/> Apri App Installer</button><p className="footnote">Se Microsoft Store non è disponibile, usa i siti ufficiali e indica i percorsi locali nei dettagli di ciascun componente.</p></div>}
      <div className="setup-tools">{setup.report.tools.map(tool => <ToolRow key={tool.name} tool={tool} setup={setup}/>)}</div>
      {setup.report.configurationError && <p className="inline-error">{setup.report.configurationError}</p>}
      <p className="footnote">Installa mancanti scarica i pacchetti WinGet indicati e ne accetta le condizioni. Windows può chiedere autorizzazioni. Serve Internet; lascia aperta l’app fino al termine.</p>
      <div className="setup-actions"><button className="text-button primary" disabled={setup.busy || setup.loading || !setup.report.winget || !missing.length} onClick={() => void setup.command('setup_install', { tool: null })}>{setup.busy ? <LoaderCircle className="spinning" size={14}/> : <Download size={14}/>} {setup.busy ? 'Configurazione in corso…' : 'Installa mancanti'}</button><button className="text-button" disabled={setup.busy} onClick={onDone}>{missing.length ? 'Più tardi' : 'Apri player'}</button></div>
    </>}
    {setup.progress?.message && <div className="setup-progress" role="status"><p>{setup.progress.message}</p>{setup.progress.completed.length > 0 && <p>Pronti: {setup.progress.completed.join(', ')}</p>}{setup.progress.errors.map((error, index) => <p className="inline-error" key={index}>{error}</p>)}</div>}
    {setup.error && <p className="inline-error" role="alert">{setup.error}</p>}
    <p className="footnote">I percorsi sono configurati per l’app e aggiornati anche se Windows aveva un PATH precedente. Nessuna variabile di sistema va copiata a mano. Account Twitch e Google sono opzionali e si collegano separatamente qui sotto.</p>
  </section>;
}

function ToolRow({ tool, setup }: { tool: ToolStatus; setup: SetupController }) {
  const [path, setPath] = useState(tool.configuredPath ?? '');
  useEffect(() => setPath(tool.configuredPath ?? ''), [tool.configuredPath]);
  return <details className="setup-tool">
    <summary><span className={tool.available ? 'dep-ok' : 'dep-bad'}>{tool.available ? <Check size={14}/> : <AlertCircle size={14}/>}</span><span><strong>{tool.name === 'ffmpeg' ? 'FFmpeg + FFprobe' : tool.name}</strong><small>{descriptions[tool.name]}</small></span><span className="setup-tool-status">{setup.progress?.running && setup.progress.current === tool.name ? 'Installazione…' : tool.available ? 'Pronto' : 'Da configurare'}</span></summary>
    {tool.path && <p className="setup-path">{tool.path}</p>}
    {!tool.available && <p className="inline-error">{tool.error}</p>}
    <p className="footnote">Pacchetto: {tool.package}</p>
    <div className="setup-actions"><button className="text-button" disabled={setup.busy || !setup.report?.winget || tool.available} onClick={() => void setup.command('setup_install', { tool: tool.name })}><Download size={12}/> Installa</button><button className="text-button" onClick={() => void setup.command('setup_open_download', { tool: tool.name })}><ExternalLink size={12}/> Sito ufficiale</button></div>
    <form className="account-form tool-path-form" onSubmit={event => { event.preventDefault(); void setup.command('setup_set_path', { tool: tool.name, path }); }}>
      <label className="field-label" htmlFor={`path-${tool.name}`}>Percorso locale facoltativo · {tool.name}.exe</label>
      <input id={`path-${tool.name}`} spellCheck={false} autoComplete="off" value={path} disabled={setup.busy} onChange={event => setPath(event.target.value)} placeholder={'C:\\Strumenti\\' + tool.name + '.exe'}/>
      <button className="text-button" disabled={setup.busy || setup.loading} type="submit">{path.trim() ? 'Verifica e salva percorso' : 'Usa rilevamento automatico'}</button>
    </form>
  </details>;
}

import { useEffect, useLayoutEffect, useRef, useState } from 'react';
import { invoke, isTauri } from '@tauri-apps/api/core';
import { listen } from '@tauri-apps/api/event';
import { ArrowDownLeft, ExternalLink, LogOut, MessageCircle, RotateCcw, Send, Twitch, Youtube } from 'lucide-react';

export type ChatMessage = { id: string; userId: string; name: string; text: string; color: string };
export type ChatState = {
  version: number; login: string | null; canSend: boolean; detached: boolean; channel: string | null; status: string; error: string;
  userCode: string | null; verificationUri: string | null; messages: ChatMessage[];
  moderation?: ChatDelta[];
};
type ChatDelta = { version: number; kind: 'message' | 'delete' | 'clearUser' | 'clear'; message?: ChatMessage; target?: string };
const initial: ChatState = { version: 0, login: null, canSend: false, detached: false, channel: null, status: 'Collega Twitch nelle impostazioni', error: '', userCode: null, verificationUri: null, messages: [] };

function mergeSnapshot(previous: ChatState, snapshot: ChatState): ChatState {
  if (snapshot.version < previous.version) return previous;
  return { ...snapshot, moderation: snapshot.channel === previous.channel && snapshot.login === previous.login ? previous.moderation : [] };
}

export function useTwitchChat(channel?: string | null) {
  const [state, setState] = useState<ChatState>(initial);
  const [error, setError] = useState('');
  useEffect(() => {
    if (!isTauri()) return;
    let disposed = false;
    const cleanup: (() => void)[] = [];
    const subscribe = async () => {
      const stopState = await listen<ChatState>('twitch-chat-state', event => {
        if (!disposed) setState(previous => mergeSnapshot(previous, event.payload));
      });
      if (disposed) { stopState(); return; }
      cleanup.push(stopState);
      const stopWindowError = await listen<string>('chat-window-error', event => { if (!disposed) setError(event.payload); });
      if (disposed) { stopWindowError(); return; }
      cleanup.push(stopWindowError);
      const stopMessages = await listen<ChatDelta>('twitch-chat-event', ({ payload }) => {
        if (disposed) return;
        setState(previous => {
          if (payload.version <= previous.version) return previous;
          let messages = previous.messages;
          switch (payload.kind) {
            case 'message': if (payload.message && !messages.some(m => m.id === payload.message!.id)) messages = [...messages, payload.message].slice(-200); break;
            case 'delete': messages = messages.filter(m => m.id !== payload.target); break;
            case 'clearUser': messages = messages.filter(m => m.userId !== payload.target); break;
            case 'clear': messages = []; break;
          }
          const moderation = payload.kind === 'message' ? previous.moderation : [...(previous.moderation ?? []), payload].slice(-1000);
          return { ...previous, version: payload.version, messages, moderation };
        });
      });
      if (disposed) { stopMessages(); return; }
      cleanup.push(stopMessages);
      const snapshot = await invoke<ChatState>('chat_get_state');
      if (!disposed) setState(previous => mergeSnapshot(previous, snapshot));
    };
    void subscribe().catch(e => { if (!disposed) setError(String(e)); });
    return () => { disposed = true; cleanup.forEach(stop => stop()); };
  }, []);
  useEffect(() => {
    // The detached chat observes the same session; only the main player selects channels.
    if (isTauri() && channel !== undefined) void invoke('chat_watch', { channel }).catch(e => setError(String(e)));
  }, [channel]);
  const command = async (name: string, args?: Record<string, unknown>) => {
    setError('');
    if (!isTauri()) { setError('La chat Twitch è disponibile nell’app desktop'); return; }
    try { await invoke(name, args); } catch (e) { setError(String(e)); }
  };
  const sendMessage = async (channel: string, message: string) => {
    if (!isTauri()) throw new Error('L’invio è disponibile nell’app desktop');
    await invoke('chat_send_message', { channel, message });
  };
  return { state, error, command, sendMessage };
}
export type TwitchChat = ReturnType<typeof useTwitchChat>;

export function ChatPanel({ chat, channel, onSettings, provider = 'twitch' }: { chat: TwitchChat; channel: string; onSettings: () => void; provider?: 'twitch' | 'youtube' }) {
  const { state } = chat;
  const youtube = provider === 'youtube';
  const service = youtube ? 'Google' : 'Twitch';
  const prefix = youtube ? 'youtube' : 'chat';
  const matchesChannel = state.channel === channel;
  const messages = matchesChannel ? state.messages : [];
  const ready = matchesChannel && state.status === 'In diretta';
  const list = useRef<HTMLDivElement>(null);
  const [displayed, setDisplayed] = useState(messages);
  const [hovered, setHovered] = useState(false);
  const [manualPause, setManualPause] = useState(false);
  const [selectionPause, setSelectionPause] = useState(false);
  const [hoverOverride, setHoverOverride] = useState(false);
  const paused = manualPause || selectionPause || (hovered && !hoverOverride);
  const pausedRef = useRef(paused);
  pausedRef.current = paused;
  const userScroll = useRef(false);
  const moderationVersion = useRef(0);
  const [draft, setDraft] = useState('');
  const [sending, setSending] = useState(false);
  const [sendError, setSendError] = useState('');
  const sendingRef = useRef(false);
  const context = useRef(0);
  useEffect(() => {
    context.current++;
    setDraft(''); setSendError('');
    return () => { context.current++; };
  }, [channel, state.login]);
  const submit = async () => {
    if (sendingRef.current || !ready || !state.canSend || !draft.trim()) return;
    const text = draft;
    const revision = context.current;
    sendingRef.current = true;
    setSending(true); setSendError('');
    try {
      await chat.sendMessage(channel, text);
      if (revision === context.current) setDraft(value => value === text ? '' : value);
    } catch (error) {
      if (revision === context.current) setSendError(error instanceof Error ? error.message : String(error));
    } finally { sendingRef.current = false; setSending(false); }
  };
  useLayoutEffect(() => {
    setManualPause(false); setSelectionPause(false); setHovered(false); setHoverOverride(false);
    userScroll.current = false; moderationVersion.current = 0;
  }, [channel, state.login]);
  useLayoutEffect(() => {
    const changes = (state.moderation ?? []).filter(event => event.version > moderationVersion.current);
    moderationVersion.current = state.version;
    if (!paused) { setDisplayed(matchesChannel ? state.messages : []); return; }
    // Freeze new rows while reading, but still honor moderation. Buffer eviction
    // in the live stream must not remove the rows currently being read.
    if (changes.length) setDisplayed(previous => changes.reduce((rows, event) => {
      if (event.kind === 'clear') return [];
      if (event.kind === 'delete') return rows.filter(row => row.id !== event.target);
      if (event.kind === 'clearUser') return rows.filter(row => row.userId !== event.target);
      return rows;
    }, previous));
  }, [state.version, state.messages, state.moderation, matchesChannel, paused]);
  useLayoutEffect(() => {
    if (!paused && list.current) list.current.scrollTop = list.current.scrollHeight;
  }, [displayed, paused]);
  useEffect(() => {
    const target = list.current;
    if (!target) return;
    const observer = new ResizeObserver(() => {
      if (!pausedRef.current) target.scrollTop = target.scrollHeight;
    });
    observer.observe(target);
    return () => observer.disconnect();
  }, [state.login]);
  useEffect(() => {
    const selectionChanged = () => {
      const selection = window.getSelection();
      const target = list.current;
      setSelectionPause(!!target && !!selection && !selection.isCollapsed &&
        (target.contains(selection.anchorNode) || target.contains(selection.focusNode)));
    };
    document.addEventListener('selectionchange', selectionChanged);
    return () => document.removeEventListener('selectionchange', selectionChanged);
  }, []);
  const resume = () => {
    userScroll.current = false;
    window.getSelection()?.removeAllRanges();
    setManualPause(false); setSelectionPause(false); setHoverOverride(true);
    setDisplayed(messages);
    requestAnimationFrame(() => { if (list.current) list.current.scrollTop = list.current.scrollHeight; });
  };
  const unseen = messages.filter(message => !displayed.some(row => row.id === message.id)).length;
  return <section className="chat-panel" aria-label={youtube ? 'Chat YouTube' : 'Chat Twitch'}>
    <div className="panel-heading"><h1>{youtube ? <Youtube size={15}/> : <Twitch size={15}/>} {youtube ? 'YouTube · chat live' : `#${channel}`}</h1>
      <div className="chat-actions"><button className="icon-button" title={state.detached ? 'Reinserisci chat nel player' : 'Apri chat in finestra'} aria-label={state.detached ? 'Reinserisci chat nel player' : 'Apri chat in finestra'} onClick={() => void chat.command(`${prefix}_set_detached`, { detached: !state.detached })}>{state.detached ? <ArrowDownLeft size={16}/> : <ExternalLink size={16}/>}</button>
      <button className="icon-button" title="Riconnetti chat" aria-label="Riconnetti chat" disabled={!state.login || !state.channel} onClick={() => void chat.command(`${prefix}_retry`)}><RotateCcw size={16}/></button></div>
    </div>
    <p className="panel-caption" role="status"><span className={`status-dot ${ready ? 'dot-live' : ''}`}/>{matchesChannel ? state.status : 'Connessione chat…'}</p>
    {matchesChannel && (state.error || chat.error) && <p className="inline-error" role="alert">{state.error || chat.error}</p>}
    {!state.login ? <div className="empty-state"><MessageCircle size={28}/><p>La chat, accanto al tuo ascolto.</p><button className="text-button" onClick={onSettings}>Collega il tuo account {service}</button></div> :
      <div className="chat-messages" ref={list} role="log" aria-label="Messaggi in diretta" aria-live="off" tabIndex={0}
      onPointerEnter={event => { if (event.pointerType === 'mouse') setHovered(true); }}
      onPointerLeave={() => { setHovered(false); setHoverOverride(false); userScroll.current = false; }}
      onWheel={event => { userScroll.current = true; if (event.deltaY < 0 && event.currentTarget.scrollHeight > event.currentTarget.clientHeight + 8) setManualPause(true); }}
      onPointerDown={() => { userScroll.current = true; }}
      onTouchStart={() => { userScroll.current = true; }}
      onKeyDown={event => { if (['ArrowUp', 'ArrowDown', 'PageUp', 'PageDown', 'Home', 'End', ' '].includes(event.key)) userScroll.current = true; }}
      onScroll={() => {
        const el = list.current;
        // Programmatic scroll and layout changes never turn live-follow off.
        if (!el || !userScroll.current) return;
        const atBottom = el.scrollHeight - el.scrollTop - el.clientHeight <= 8;
        setManualPause(!atBottom);
        if (atBottom) setHoverOverride(true);
      }}>
        {!displayed.length && <div className="empty-state"><p>In attesa di nuovi messaggi</p></div>}
        {displayed.map(message => <p className="chat-message" key={message.id}><strong style={{ color: /^#[0-9a-f]{6}$/i.test(message.color) ? message.color : undefined }}>{message.name}</strong><span>{message.text}</span></p>)}
      </div>}
    {paused && <button className="follow-chat text-button" onPointerEnter={() => setHovered(true)} onPointerLeave={() => setHovered(false)} onClick={resume}>Scorrimento in pausa{unseen ? ` · ${unseen >= 200 ? '200+' : unseen} nuovi` : ''} · Torna al live ↓</button>}
    {state.login && <>
      {!state.canSend && <button className="text-button chat-upgrade" onClick={onSettings}>Autorizza l’invio nelle impostazioni</button>}
      <form className="chat-composer" onSubmit={event => { event.preventDefault(); void submit(); }}>
        <input aria-label={youtube ? 'Messaggio alla chat YouTube' : `Messaggio a #${channel}`} value={draft} maxLength={youtube ? 200 : 500} disabled={!ready || !state.canSend} placeholder={state.canSend ? `Scrivi come ${state.login}…` : `Ricollega ${service} per scrivere…`} onChange={event => setDraft(event.target.value)} onKeyDown={event => { if (event.key === 'Enter' && event.nativeEvent.isComposing) event.preventDefault(); }}/>
        <button className="icon-button" type="submit" disabled={!ready || !state.canSend || sending || !draft.trim()} aria-label="Invia messaggio" title="Invia messaggio"><Send size={16}/></button>
      </form>
      {sendError && <p className="inline-error send-error" role="alert">{sendError}</p>}
    </>}
    <div className="chat-footnote">Ultimi 200 messaggi · {youtube ? 'aggiornamenti secondo i tempi YouTube' : 'la chat può anticipare l’audio'}</div>
  </section>;
}

export function TwitchSettings({ chat }: { chat: TwitchChat }) {
  const [clientId, setClientId] = useState(() => localStorage.getItem('background-audio:twitch-client-id') || '');
  const { state } = chat;
  const pending = state.status === 'Accesso in corso' || !!state.userCode;
  return <section className="settings-group">
    <h2><Twitch size={16}/> Account Twitch</h2>
    {state.login ? <><div className="account-row"><span>Collegato come <strong>{state.login}</strong></span><button className="text-button" onClick={() => void chat.command('chat_logout')}><LogOut size={14}/> Scollega</button></div>
      {!state.canSend && <div className="chat-permission"><p className="footnote">Per scrivere serve anche il permesso di invio. Ricollega Twitch e autorizza lettura e scrittura.</p><button className="text-button primary" disabled={!clientId.trim()} onClick={() => void chat.command('chat_login', { clientId: clientId.trim() })}>Abilita invio messaggi</button></div>}</> : <>
      <p className="footnote">Collega Twitch per leggere e scrivere in chat. La riproduzione funziona anche senza account.</p>
      <label className="field-label" htmlFor="twitch-client-id">Client ID</label>
      <form className="account-form" onSubmit={event => {
        event.preventDefault(); localStorage.setItem('background-audio:twitch-client-id', clientId.trim());
        void chat.command('chat_login', { clientId: clientId.trim() });
      }}>
        <input id="twitch-client-id" autoComplete="off" spellCheck={false} value={clientId} disabled={pending} onChange={event => setClientId(event.target.value)} placeholder="Client ID della tua app Twitch"/>
        <button className="text-button primary" type="submit" disabled={pending || !clientId.trim()}>{pending ? 'In attesa…' : 'Collega Twitch'}</button>
      </form>
      <details><summary>Come ottengo il Client ID?</summary><p className="footnote">Nella console sviluppatori Twitch registra un’app con tipo client <strong>Public</strong>, categoria applicazione desktop e redirect <code>http://localhost</code>. Copia qui il Client ID. Non serve un Client Secret.</p><button className="text-button" onClick={() => void chat.command('chat_open_developer_console')}><ExternalLink size={13}/> Console sviluppatori Twitch</button></details>
    </>}
    {state.userCode && <div className="activation"><span>Autorizza Twitch nel browser</span><strong>{state.userCode}</strong><button className="text-button primary" onClick={() => void chat.command('chat_open_activation')}><ExternalLink size={14}/> Apri Twitch</button></div>}
    {pending && <button className="text-button" onClick={() => void chat.command('chat_logout')}>Annulla accesso</button>}
    <p className="footnote">Account ricordato in Gestione credenziali Windows. Scollega rimuove l’accesso salvato. Lettura e invio messaggi; i messaggi partono soltanto quando premi Invia.</p>
    {(state.error || chat.error) && <p className="inline-error" role="alert">{state.error || chat.error}</p>}
  </section>;
}

export function DetachedChat() {
  const chat = useTwitchChat();
  return <main className="detached-chat">
    {chat.state.channel ? <ChatPanel key={chat.state.channel} channel={chat.state.channel} chat={chat} onSettings={() => void chat.command('chat_open_settings')}/> :
      <div className="empty-state"><MessageCircle size={28}/><p>Avvia un canale Twitch nel player.</p><button className="text-button" onClick={() => void chat.command('chat_set_detached', { detached: false })}>Torna al player</button></div>}
  </main>;
}

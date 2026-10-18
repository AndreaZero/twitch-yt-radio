import { useEffect, useRef, useState } from 'react';
import { invoke, isTauri } from '@tauri-apps/api/core';
import { listen } from '@tauri-apps/api/event';
import { ExternalLink, LogOut, MessageCircle, RotateCcw, Send, Twitch } from 'lucide-react';

type ChatMessage = { id: string; userId: string; name: string; text: string; color: string };
type ChatState = {
  version: number; login: string | null; canSend: boolean; channel: string | null; status: string; error: string;
  userCode: string | null; verificationUri: string | null; messages: ChatMessage[];
};
type ChatDelta = { version: number; kind: 'message' | 'delete' | 'clearUser' | 'clear'; message?: ChatMessage; target?: string };
const initial: ChatState = { version: 0, login: null, canSend: false, channel: null, status: 'Collega Twitch nelle impostazioni', error: '', userCode: null, verificationUri: null, messages: [] };

export function useTwitchChat(channel: string | null) {
  const [state, setState] = useState<ChatState>(initial);
  const [error, setError] = useState('');
  useEffect(() => {
    if (!isTauri()) return;
    let disposed = false;
    const cleanup: (() => void)[] = [];
    const subscribe = async () => {
      const stopState = await listen<ChatState>('twitch-chat-state', event => {
        if (!disposed) setState(previous => event.payload.version >= previous.version ? event.payload : previous);
      });
      if (disposed) { stopState(); return; }
      cleanup.push(stopState);
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
          return { ...previous, version: payload.version, messages };
        });
      });
      if (disposed) { stopMessages(); return; }
      cleanup.push(stopMessages);
      const snapshot = await invoke<ChatState>('chat_get_state');
      if (!disposed) setState(previous => snapshot.version >= previous.version ? snapshot : previous);
    };
    void subscribe().catch(e => { if (!disposed) setError(String(e)); });
    return () => { disposed = true; cleanup.forEach(stop => stop()); };
  }, []);
  useEffect(() => {
    if (isTauri()) void invoke('chat_watch', { channel }).catch(e => setError(String(e)));
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

export function ChatPanel({ chat, channel, onSettings }: { chat: TwitchChat; channel: string; onSettings: () => void }) {
  const { state } = chat;
  const matchesChannel = state.channel === channel;
  const messages = matchesChannel ? state.messages : [];
  const ready = matchesChannel && state.status === 'In diretta';
  const list = useRef<HTMLDivElement>(null);
  const follow = useRef(true);
  const [following, setFollowing] = useState(true);
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
  useEffect(() => {
    if (follow.current && list.current) list.current.scrollTop = list.current.scrollHeight;
  }, [state.messages]);
  useEffect(() => { follow.current = true; setFollowing(true); }, [state.channel]);
  return <section className="chat-panel" aria-label="Chat Twitch">
    <div className="panel-heading"><h1><Twitch size={15}/>{`#${channel}`}</h1>
      <button className="icon-button" title="Riconnetti chat" aria-label="Riconnetti chat" disabled={!state.login || !state.channel} onClick={() => void chat.command('chat_retry')}><RotateCcw size={16}/></button>
    </div>
    <p className="panel-caption" role="status"><span className={`status-dot ${ready ? 'dot-live' : ''}`}/>{matchesChannel ? state.status : 'Connessione chat…'}</p>
    {matchesChannel && (state.error || chat.error) && <p className="inline-error" role="alert">{state.error || chat.error}</p>}
    {!state.login ? <div className="empty-state"><MessageCircle size={28}/><p>La chat, accanto al tuo ascolto.</p><button className="text-button" onClick={onSettings}>Collega il tuo account Twitch</button></div> :
      <div className="chat-messages" ref={list} role="log" aria-label="Messaggi in diretta" aria-live="off" onScroll={() => {
        const el = list.current;
        if (!el) return;
        follow.current = el.scrollHeight - el.scrollTop - el.clientHeight < 45;
        setFollowing(follow.current);
      }}>
        {!messages.length && <div className="empty-state"><p>In attesa di nuovi messaggi</p></div>}
        {messages.map(message => <p className="chat-message" key={message.id}><strong style={{ color: /^#[0-9a-f]{6}$/i.test(message.color) ? message.color : undefined }}>{message.name}</strong><span>{message.text}</span></p>)}
      </div>}
    {!following && <button className="follow-chat text-button" onClick={() => { follow.current = true; setFollowing(true); if (list.current) list.current.scrollTop = list.current.scrollHeight; }}>Torna ai messaggi live ↓</button>}
    {state.login && <>
      {!state.canSend && <button className="text-button chat-upgrade" onClick={onSettings}>Autorizza l’invio nelle impostazioni</button>}
      <form className="chat-composer" onSubmit={event => { event.preventDefault(); void submit(); }}>
        <input aria-label={`Messaggio a #${channel}`} value={draft} maxLength={500} disabled={!ready || !state.canSend} placeholder={state.canSend ? `Scrivi come ${state.login}…` : 'Ricollega Twitch per scrivere…'} onChange={event => setDraft(event.target.value)} onKeyDown={event => { if (event.key === 'Enter' && event.nativeEvent.isComposing) event.preventDefault(); }}/>
        <button className="icon-button" type="submit" disabled={!ready || !state.canSend || sending || !draft.trim()} aria-label="Invia messaggio" title="Invia messaggio"><Send size={16}/></button>
      </form>
      {sendError && <p className="inline-error send-error" role="alert">{sendError}</p>}
    </>}
    <div className="chat-footnote">Ultimi 200 messaggi · la chat può anticipare l’audio</div>
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
    <p className="footnote">Accesso per questa sessione. Token solo in memoria; permessi di lettura e invio messaggi. I messaggi partono soltanto quando premi Invia.</p>
    {(state.error || chat.error) && <p className="inline-error" role="alert">{state.error || chat.error}</p>}
  </section>;
}

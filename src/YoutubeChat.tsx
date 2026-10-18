import { useEffect, useRef, useState } from 'react';
import { invoke, isTauri } from '@tauri-apps/api/core';
import { listen } from '@tauri-apps/api/event';
import { ArrowDownLeft, ExternalLink, LogOut, MessageCircle, RotateCcw, Send, Youtube } from 'lucide-react';
import { ChatPanel } from './TwitchChat';
import type { ChatMessage, ChatState } from './TwitchChat';

type Comment = ChatMessage & { publishedAt: string; likeCount: number; replyCount: number };
type YoutubeState = Omit<ChatState, 'messages'> & {
  mode: 'idle' | 'live' | 'comments' | 'unavailable'; canLoadMore: boolean; loading: boolean; messages: Comment[];
};
const initial: YoutubeState = {
  version: 0, login: null, canSend: false, detached: false, channel: null,
  status: 'Collega Google nelle impostazioni', error: '', userCode: null, verificationUri: null,
  messages: [], mode: 'idle', canLoadMore: false, loading: false,
};

export function useYoutubeChat(url?: string | null) {
  const [state, setState] = useState<YoutubeState>(initial);
  const [error, setError] = useState('');
  useEffect(() => {
    if (!isTauri()) return;
    let disposed = false;
    const cleanup: (() => void)[] = [];
    const merge = (snapshot: YoutubeState) => {
      if (!disposed) setState(previous => snapshot.version >= previous.version ? snapshot : previous);
    };
    const subscribe = async () => {
      const stopState = await listen<YoutubeState>('youtube-state', event => merge(event.payload));
      if (disposed) { stopState(); return; }
      cleanup.push(stopState);
      const stopError = await listen<string>('youtube-window-error', event => { if (!disposed) setError(event.payload); });
      if (disposed) { stopError(); return; }
      cleanup.push(stopError);
      merge(await invoke<YoutubeState>('youtube_get_state'));
    };
    void subscribe().catch(e => { if (!disposed) setError(String(e)); });
    return () => { disposed = true; cleanup.forEach(stop => stop()); };
  }, []);
  useEffect(() => {
    // A popout observes the session; it never changes the selected video.
    if (isTauri() && url !== undefined) {
      setError('');
      void invoke('youtube_watch', { url }).catch(e => setError(String(e)));
    }
  }, [url]);
  const command = async (name: string, args?: Record<string, unknown>) => {
    setError('');
    if (!isTauri()) { setError('YouTube è disponibile nell’app desktop'); return; }
    try { await invoke(name, args); } catch (e) { setError(String(e)); }
  };
  const sendMessage = async (channel: string, message: string) => {
    if (!isTauri()) throw new Error('L’invio è disponibile nell’app desktop');
    await invoke('youtube_send_message', { channel, message });
  };
  return { state, error, command, sendMessage };
}
type YoutubeChat = ReturnType<typeof useYoutubeChat>;

export function YoutubePanel({ chat, onSettings }: { chat: YoutubeChat; onSettings: () => void }) {
  const { state } = chat;
  if (state.mode === 'live' && state.channel) {
    return <ChatPanel key={state.channel} provider="youtube" channel={state.channel} chat={chat} onSettings={onSettings}/>;
  }
  if (state.mode === 'comments' && state.channel) return <CommentsPanel key={state.channel} chat={chat}/>;
  return <section className="chat-panel" aria-label="YouTube">
    <PanelHeader chat={chat}/>
    <p className="panel-caption" role="status">{state.status}</p>
    {(state.error || chat.error) && <p className="inline-error" role="alert">{state.error || chat.error}</p>}
    <div className="empty-state"><MessageCircle size={24}/>
      {!state.login ? <button className="text-button" onClick={onSettings}>Collega Google per chat e commenti</button> :
        <p>{state.loading ? 'Caricamento…' : 'Chat live o commenti seguono il video selezionato.'}</p>}
    </div>
  </section>;
}

function PanelHeader({ chat, comments = false }: { chat: YoutubeChat; comments?: boolean }) {
  const { state } = chat;
  return <div className="panel-heading"><h1><Youtube size={15}/> YouTube · {comments ? 'commenti' : 'community'}</h1>
    <div className="chat-actions"><button className="icon-button" title={state.detached ? 'Reinserisci nel player' : 'Apri in finestra'} aria-label={state.detached ? 'Reinserisci nel player' : 'Apri in finestra'} onClick={() => void chat.command('youtube_set_detached', { detached: !state.detached })}>{state.detached ? <ArrowDownLeft size={16}/> : <ExternalLink size={16}/>}</button>
      <button className="icon-button" title={comments ? 'Aggiorna commenti' : 'Riconnetti YouTube'} aria-label={comments ? 'Aggiorna commenti' : 'Riconnetti YouTube'} disabled={state.loading || !state.login || !state.channel} onClick={() => void chat.command('youtube_retry')}><RotateCcw size={16}/></button>
    </div>
  </div>;
}

function CommentsPanel({ chat }: { chat: YoutubeChat }) {
  const { state } = chat;
  const [draft, setDraft] = useState('');
  const [sending, setSending] = useState(false);
  const [sendError, setSendError] = useState('');
  const lock = useRef(false);
  const context = useRef(0);
  useEffect(() => {
    context.current++; setDraft(''); setSendError('');
    return () => { context.current++; };
  }, [state.channel, state.login]);
  const submit = async () => {
    if (lock.current || !state.channel || !state.canSend || !draft.trim()) return;
    const revision = context.current;
    const text = draft;
    lock.current = true; setSending(true); setSendError('');
    try {
      await chat.sendMessage(state.channel, text);
      if (revision === context.current) setDraft(previous => previous === text ? '' : previous);
    } catch (e) {
      if (revision === context.current) setSendError(String(e));
    } finally { lock.current = false; setSending(false); }
  };
  return <section className="chat-panel comments-panel" aria-label="Commenti YouTube">
    <PanelHeader chat={chat} comments/>
    <p className="panel-caption" role="status">{state.loading ? 'Caricamento commenti…' : state.status}</p>
    {(state.error || chat.error) && <p className="inline-error" role="alert">{state.error || chat.error}</p>}
    <div className="chat-messages comment-list" tabIndex={0} aria-label="Commenti, dal più recente">
      {!state.messages.length && <div className="empty-state"><p>{state.loading ? 'Caricamento…' : 'Nessun commento disponibile'}</p></div>}
      {state.messages.map(comment => <article className="video-comment" key={comment.id}>
        <div className="comment-meta"><strong>{comment.name}</strong><time dateTime={comment.publishedAt}>{comment.publishedAt ? new Date(comment.publishedAt).toLocaleDateString('it-IT') : ''}</time></div>
        <p>{comment.text}</p>
        {(comment.likeCount > 0 || comment.replyCount > 0) && <small>{comment.likeCount > 0 ? `${comment.likeCount} Mi piace` : ''}{comment.likeCount > 0 && comment.replyCount > 0 ? ' · ' : ''}{comment.replyCount > 0 ? `${comment.replyCount} risposte su YouTube` : ''}</small>}
      </article>)}
      {state.canLoadMore && <button className="text-button load-comments" disabled={state.loading} onClick={() => void chat.command('youtube_more_comments')}>{state.loading ? 'Caricamento…' : 'Carica altri commenti'}</button>}
    </div>
    <form className="comment-composer" onSubmit={event => { event.preventDefault(); void submit(); }}>
      <textarea aria-label="Nuovo commento pubblico YouTube" maxLength={10000} rows={2} disabled={!state.canSend} value={draft} onChange={event => setDraft(event.target.value)} placeholder={state.canSend ? `Commenta come ${state.login}…` : 'Commenti non disponibili'}/>
      <button className="text-button" type="submit" disabled={!state.canSend || sending || !draft.trim()}><Send size={13}/>{sending ? 'Invio…' : 'Pubblica commento'}</button>
    </form>
    {sendError && <p className="inline-error send-error" role="alert">{sendError}</p>}
    <div className="chat-footnote">Commenti pubblici · più recenti · aggiorna con ↻</div>
  </section>;
}

export function YoutubeSettings({ chat }: { chat: YoutubeChat }) {
  const [clientId, setClientId] = useState(() => localStorage.getItem('background-audio:google-client-id') || '');
  const [secret, setSecret] = useState('');
  const { state } = chat;
  const pending = state.status === 'Accesso Google in corso';
  return <section className="settings-group">
    <h2><Youtube size={16}/> Account Google · YouTube</h2>
    {state.login && <div className="account-row"><span>Collegato come <strong>{state.login}</strong></span><button className="text-button" onClick={() => void chat.command('youtube_logout')}><LogOut size={14}/> Scollega</button></div>}
    <p className="footnote">Lettura e invio nella chat live e nei commenti dei video. La riproduzione funziona anche senza collegare Google.</p>
    <form className="account-form" onSubmit={event => {
      event.preventDefault();
      localStorage.setItem('background-audio:google-client-id', clientId.trim());
      void chat.command('youtube_login', { clientId: clientId.trim(), clientSecret: secret.trim() });
      setSecret('');
    }}>
      <label className="field-label" htmlFor="google-client-id">Client ID OAuth · App desktop</label>
      <input id="google-client-id" autoComplete="off" spellCheck={false} value={clientId} disabled={pending} onChange={event => setClientId(event.target.value)} placeholder="…apps.googleusercontent.com"/>
      <label className="field-label" htmlFor="google-client-secret">Client Secret del client desktop</label>
      <input id="google-client-secret" type="password" autoComplete="off" value={secret} disabled={pending} onChange={event => setSecret(event.target.value)} placeholder="Client Secret da Google Cloud"/>
      <button className="text-button primary" type="submit" disabled={pending || !clientId.trim() || !secret.trim()}>{pending ? 'Completa l’accesso nel browser…' : state.login ? 'Ricollega / cambia account Google' : 'Collega Google'}</button>
    </form>
    {pending && <button className="text-button" onClick={() => void chat.command('youtube_logout')}>Annulla accesso</button>}
    <details><summary>Configurazione Google Cloud</summary>
      <p className="footnote">Abilita <strong>YouTube Data API v3</strong>, configura la schermata consenso e crea un client OAuth di tipo <strong>App desktop</strong>. In modalità Testing aggiungi il tuo account tra gli utenti di test. Copia Client ID e Client Secret qui e autorizza l’accesso a YouTube nel browser.</p>
      <button className="text-button" onClick={() => void chat.command('youtube_open_console')}><ExternalLink size={13}/> Apri Google Cloud</button>
    </details>
    <p className="footnote">Account ricordato in Gestione credenziali Windows con rinnovo automatico. In Testing Google può far scadere l’autorizzazione dopo 7 giorni. Scollega rimuove la sessione locale; per revocarla anche in Google usa le connessioni del tuo account.</p>
    <p className="footnote">Chat live aggiornata secondo gli intervalli YouTube; il progetto consuma quota API. Commenti caricati su richiesta, fino a 1.000 per video; le risposte ai commenti restano su YouTube.</p>
    {(state.error || chat.error) && <p className="inline-error" role="alert">{state.error || chat.error}</p>}
  </section>;
}

export function DetachedYoutubeChat() {
  const chat = useYoutubeChat();
  return <main className="detached-chat"><YoutubePanel chat={chat} onSettings={() => void chat.command('chat_open_settings')}/></main>;
}

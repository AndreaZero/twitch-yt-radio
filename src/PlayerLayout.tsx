import { useEffect, useLayoutEffect, useRef, useState } from 'react';
import type { CSSProperties, PointerEvent, ReactNode } from 'react';

const KEY = 'background-audio:player-share:v1';
function savedShare() {
  const raw = localStorage.getItem(KEY);
  const value = raw === null ? NaN : Number(raw);
  return Number.isFinite(value) && value >= 0 && value <= 1 ? value : null;
}

export function PlayerLayout({ children, community, video, withChat, detached, fullscreen, hidden }: {
  children: ReactNode; community: ReactNode; video: boolean; withChat: boolean; detached: boolean; fullscreen: boolean; hidden: boolean;
}) {
  const element = useRef<HTMLElement>(null);
  const [share, setShare] = useState<number | null>(savedShare);
  const [size, setSize] = useState({ width: 426, height: 330 });
  const [dragging, setDragging] = useState(false);
  const drag = useRef<{ pointer: number; top: number; height: number; offset: number } | null>(null);
  const resizable = video && withChat && !detached && !fullscreen;
  const total = Math.max(1, size.height - 12);
  const minPlayer = Math.min(90, total * .35);
  const minChat = Math.min(170, total * .45);
  const maxPlayer = total - minChat;
  const height = Math.min(maxPlayer, Math.max(minPlayer, share === null ? Math.min(size.width * 9 / 16, total * .66) : total * share));
  const actualShare = height / total;
  useEffect(() => {
    if (share === null) localStorage.removeItem(KEY); else localStorage.setItem(KEY, String(share));
  }, [share]);
  useLayoutEffect(() => {
    const target = element.current;
    if (!target || hidden) return;
    const measure = () => {
      const style = getComputedStyle(target);
      setSize({ width: Math.max(1, target.clientWidth - parseFloat(style.paddingLeft) - parseFloat(style.paddingRight)), height: Math.max(1, target.clientHeight - parseFloat(style.paddingTop) - parseFloat(style.paddingBottom)) });
    };
    const observer = new ResizeObserver(measure);
    observer.observe(target); measure();
    return () => observer.disconnect();
  }, [hidden, resizable]);
  useEffect(() => {
    if (!resizable || hidden) { drag.current = null; setDragging(false); }
  }, [resizable, hidden]);
  const start = (event: PointerEvent<HTMLDivElement>) => {
    if (event.button !== 0 || !element.current) return;
    event.preventDefault();
    const rect = element.current.getBoundingClientRect();
    const padding = parseFloat(getComputedStyle(element.current).paddingTop);
    drag.current = { pointer: event.pointerId, top: rect.top + padding, height: total, offset: event.clientY - (rect.top + padding + height) };
    event.currentTarget.setPointerCapture(event.pointerId); setDragging(true);
  };
  const stop = (event: PointerEvent<HTMLDivElement>) => {
    if (drag.current?.pointer !== event.pointerId) return;
    drag.current = null; setDragging(false);
    if (event.currentTarget.hasPointerCapture(event.pointerId)) event.currentTarget.releasePointerCapture(event.pointerId);
  };
  return <section ref={element} className={`player-page ${withChat ? 'with-chat' : ''} ${resizable ? 'resizable-player' : ''} ${detached ? 'video-detached' : ''} ${dragging ? 'resizing-player' : ''}`} hidden={hidden} aria-label="Player" style={{ '--player-height': `${height}px` } as CSSProperties}>
    {children}
    {resizable && <div className="player-divider" role="separator" aria-label="Dimensione player e chat" aria-orientation="horizontal" aria-valuemin={Math.round(minPlayer / total * 100)} aria-valuemax={Math.round(maxPlayer / total * 100)} aria-valuenow={Math.round(actualShare * 100)} aria-valuetext={`Player ${Math.round(actualShare * 100)}%, chat ${Math.round((1 - actualShare) * 100)}%`} tabIndex={0} title="Trascina per ridimensionare · doppio clic per automatico"
      onPointerDown={start} onPointerMove={event => {
        const current = drag.current;
        if (current?.pointer === event.pointerId) setShare(Math.min(maxPlayer / current.height, Math.max(minPlayer / current.height, (event.clientY - current.top - current.offset) / current.height)));
      }} onPointerUp={stop} onPointerCancel={stop} onLostPointerCapture={() => { drag.current = null; setDragging(false); }} onDoubleClick={() => setShare(null)}
      onKeyDown={event => {
        if (event.key === 'Enter') { event.preventDefault(); setShare(null); return; }
        if (!['ArrowUp', 'ArrowDown', 'Home', 'End'].includes(event.key)) return;
        event.preventDefault();
        setShare(event.key === 'Home' ? minPlayer / total : event.key === 'End' ? maxPlayer / total : Math.min(maxPlayer / total, Math.max(minPlayer / total, actualShare + (event.key === 'ArrowDown' ? .04 : -.04))));
      }}><span/></div>}
    {community && <div className="community-slot">{community}</div>}
  </section>;
}

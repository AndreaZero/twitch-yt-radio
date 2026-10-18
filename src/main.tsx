import React from 'react';
import ReactDOM from 'react-dom/client';
import App from './App';
import { isTauri } from '@tauri-apps/api/core';
import { getCurrentWindow } from '@tauri-apps/api/window';
import { DetachedPlayer } from './VideoPanel';
import './styles.css';

ReactDOM.createRoot(document.getElementById('root')!).render(
  <React.StrictMode>
    {isTauri() && getCurrentWindow().label === 'video' ? <DetachedPlayer/> : <App />}
  </React.StrictMode>,
);

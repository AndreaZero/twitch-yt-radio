import React from 'react';
import ReactDOM from 'react-dom/client';
import App from './App';
import { isTauri } from '@tauri-apps/api/core';
import { getCurrentWindow } from '@tauri-apps/api/window';
import { DetachedPlayer } from './VideoPanel';
import { DetachedChat } from './TwitchChat';
import { VideoControls } from './VideoControls';
import './styles.css';

const windowLabel = isTauri() ? getCurrentWindow().label : 'main';
document.documentElement.dataset.surface = windowLabel.startsWith('controls-') ? 'video-controls' : windowLabel;

ReactDOM.createRoot(document.getElementById('root')!).render(
  <React.StrictMode>
    {windowLabel.startsWith('controls-') ? <VideoControls/> : windowLabel === 'video' ? <DetachedPlayer/> : windowLabel === 'chat' ? <DetachedChat/> : <App/>}
  </React.StrictMode>,
);

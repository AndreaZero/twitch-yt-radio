export type PlaybackMode = 'audio_only' | 'video_audio';
export type Snapshot = {
  active: boolean;
  currentUrl: string | null;
  generation: number;
  paused: boolean;
  volume: number;
  mode: PlaybackMode;
};
export type EndEvent = { generation: number; failed: boolean };

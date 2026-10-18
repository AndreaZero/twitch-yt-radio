# YouTube chat and comments

[English](YOUTUBE-CHAT.en.md) | [Italiano](YOUTUBE-CHAT.it.md)

Source implementation dated October 18, 2026. **No builds, tests or account/API sessions were run for this change**, as requested.

## Connect Google

1. Select a project in [Google Cloud](https://console.cloud.google.com/apis/credentials) and enable **YouTube Data API v3**.
2. Configure OAuth consent; add your account as a test user in Testing mode.
3. Create a **Desktop app** OAuth client. Enter its Client ID and Client Secret in settings.
4. Select **Collega Google**, choose the Google/YouTube identity in your system browser and grant access. The local callback expires after three minutes.

The app requests `youtube.force-ssl` using Google's [desktop OAuth flow](https://developers.google.com/identity/protocols/oauth2/native-app). A YouTube channel is required; the displayed identity is its name, not your email.

Tokens and the desktop secret are saved in Windows Credential Manager under `dev.andreazero.backgroundaudio/google`, separately from Twitch. Refresh happens when a request needs it. Disconnect removes local credentials; revoke remote authorization in your Google account connections. Revocation, Workspace policies or [seven-day Testing refresh-token expiration](https://developers.google.com/identity/protocols/oauth2#expiration) can require consent again.

## Behavior and limits

- Validated video links: `watch?v=`, `youtu.be`, `/live/`, `/shorts/`, `/embed/`. Channel links do not identify a particular chat.
- `videos.list` selects active live chat, scheduled/chatless live status or ordinary video comments. Works in audio-only mode without changing MPV.
- Live: reading, sending up to 200 characters, deduplication, recent 200 messages and API moderation removals. Reuses Twitch's follow-to-bottom, hover/selection/manual-scroll pause and resume. A frozen list may retain another 200 rows while reading.
- Ordinary videos/recordings: newest top-level comments, 50 per page, up to 1,000 loaded, manual refresh and new public comments (local limit 10,000 characters). Like/reply counts shown; replies, editing, deleting and reactions remain on YouTube. Comments do not auto-scroll while reading.
- Separate community window shares Rust state; closing docks it. Google sign-in is separate from external playback authentication.
- Writes require explicit submit, prevent double clicks and recheck account/video before POST. No automatic write retries. An ambiguous timeout preserves the draft and asks you to check publication. YouTube restrictions/moderation can reject or delay visibility.

This version uses [liveChatMessages.list](https://developers.google.com/youtube/data/live/reference/rest/v3/liveChatMessages/list), waiting at least `pollingIntervalMillis`, with a five-second local minimum. It is polling, not Twitch-style push. Five quota units every five seconds means about 3,600 units/hour before other requests. Longer API intervals reduce usage; quota is shared by the Google Cloud project. Exhausted quota stops requests with an error. [gRPC streamList](https://developers.google.com/youtube/data/live/streaming-live-chat) is a future optimization, not implemented here.

[Video comments](https://developers.google.com/youtube/data/reference/rest/v3/commentThreads/list) load on request. Scheduled/chatless streams are checked about every 30–60 seconds. Network read failures use increasing backoff; permission/quota/disabled-content failures require user action. No chat replay after live ends, or synchronization with playback position.

## Pending Windows checks

Browser sign-in/cancel, restart persistence, refresh/revocation, correct YouTube identity, live reading/sending, hover/scroll/selection and popout, moderation, rapid source switches/stop during requests, disabled comments, comment pagination/posting, offline network and exhausted quota. The user will build and run these checks.

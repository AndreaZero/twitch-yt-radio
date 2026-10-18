use serde::{Deserialize, Serialize};

// Reserve an exit code for closing the mini video window, distinct from EOF/errors.
pub const VIDEO_CLOSED_EXIT_CODE: i32 = 10;
pub const VIDEO_INPUT: &str = "CLOSE_WIN quit 10\n";

#[derive(Clone, Copy, Default, Deserialize, Serialize, PartialEq, Debug)]
#[serde(rename_all = "snake_case")]
pub enum PlaybackMode {
    #[default]
    AudioOnly,
    VideoAudio,
}

impl PlaybackMode {
    pub fn twitch_quality(self) -> &'static str {
        match self {
            Self::AudioOnly => "audio_only",
            Self::VideoAudio => "best",
        }
    }

    pub fn youtube_format(self) -> &'static str {
        match self {
            Self::AudioOnly => "bestaudio",
            Self::VideoAudio => "bestvideo[height<=720]+bestaudio/best[height<=720]",
        }
    }

    pub fn window_args(self) -> &'static [&'static str] {
        match self {
            Self::AudioOnly => &["--no-video", "--force-window=no"],
            Self::VideoAudio => &[
                "--vid=auto",
                "--force-window=yes",
                "--autofit=384x216",
                "--ontop=yes",
                "--title=Background Audio - Video",
            ],
        }
    }
}

pub fn manually_closed(mode: PlaybackMode, code: Option<i32>) -> bool {
    mode == PlaybackMode::VideoAudio && code == Some(VIDEO_CLOSED_EXIT_CODE)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn audio_only_never_requests_or_displays_video() {
        let mode = PlaybackMode::default();
        assert_eq!(mode.twitch_quality(), "audio_only");
        assert_eq!(mode.youtube_format(), "bestaudio");
        assert!(mode.window_args().contains(&"--no-video"));
        assert!(mode.window_args().contains(&"--force-window=no"));
    }

    #[test]
    fn video_explicitly_requests_video_and_a_compact_window() {
        let mode: PlaybackMode = serde_json::from_str("\"video_audio\"").unwrap();
        assert_eq!(mode.twitch_quality(), "best");
        assert_eq!(
            mode.youtube_format(),
            "bestvideo[height<=720]+bestaudio/best[height<=720]"
        );
        assert!(mode.window_args().contains(&"--force-window=yes"));
        assert!(mode.window_args().contains(&"--autofit=384x216"));
        assert!(!mode.window_args().contains(&"--no-video"));
        assert!(serde_json::from_str::<PlaybackMode>("\"unknown\"").is_err());
    }

    #[test]
    fn video_close_is_distinct_from_eof_and_errors() {
        assert!(manually_closed(PlaybackMode::VideoAudio, Some(10)));
        assert!(!manually_closed(PlaybackMode::VideoAudio, Some(0)));
        assert!(!manually_closed(PlaybackMode::VideoAudio, Some(2)));
        assert!(!manually_closed(PlaybackMode::VideoAudio, None));
        assert!(!manually_closed(PlaybackMode::AudioOnly, Some(10)));
        assert_eq!(
            VIDEO_INPUT,
            format!("CLOSE_WIN quit {VIDEO_CLOSED_EXIT_CODE}\n")
        );
    }
}

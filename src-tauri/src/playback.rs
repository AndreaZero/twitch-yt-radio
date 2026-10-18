use serde::{Deserialize, Serialize};

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
                "--input-vo-keyboard=no",
                "--input-cursor=no",
            ],
        }
    }
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
    fn video_explicitly_requests_video_with_controls_owned_by_the_widget() {
        let mode: PlaybackMode = serde_json::from_str("\"video_audio\"").unwrap();
        assert_eq!(mode.twitch_quality(), "best");
        assert_eq!(
            mode.youtube_format(),
            "bestvideo[height<=720]+bestaudio/best[height<=720]"
        );
        assert!(mode.window_args().contains(&"--force-window=yes"));
        assert!(mode.window_args().contains(&"--input-vo-keyboard=no"));
        assert!(!mode.window_args().contains(&"--no-video"));
        assert!(serde_json::from_str::<PlaybackMode>("\"unknown\"").is_err());
    }
}

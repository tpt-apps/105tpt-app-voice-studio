//! Subtitle/caption generation (SRT/VTT) from the edited transcript
//! (spec §11).
//!
//! Cues are built from the rendered output timeline, so captions always
//! match the edited audio. Words never span cues across segment boundaries,
//! and cue length is bounded by configurable duration/line limits.

use std::collections::HashMap;
use std::time::Duration;

use tpt_app_voice_studio_core::id::{SegmentId, SpeakerId};
use tpt_app_voice_studio_core::time::TimeRange;
use tpt_app_voice_studio_edit::timeline::render_timeline;
use tpt_app_voice_studio_model::export::SubtitleFormat;
use tpt_app_voice_studio_model::speaker::Speaker;
use tpt_app_voice_studio_model::transcript::Transcript;

/// Options controlling cue packing and content.
#[derive(Clone, Debug)]
pub struct SubtitleOptions {
    /// Output format.
    pub format: SubtitleFormat,
    /// Maximum characters per caption line.
    pub max_chars_per_line: usize,
    /// Maximum lines per cue.
    pub max_lines: usize,
    /// Maximum rendered duration of one cue.
    pub max_cue_duration: Duration,
    /// Prefix cues that start a segment with the speaker's label
    /// (`"Host: ..."`), useful for multi-speaker captioning.
    pub include_speaker_labels: bool,
}

impl Default for SubtitleOptions {
    fn default() -> Self {
        Self {
            format: SubtitleFormat::Srt,
            max_chars_per_line: 42,
            max_lines: 2,
            max_cue_duration: Duration::from_secs(5),
            include_speaker_labels: false,
        }
    }
}

/// One caption cue on the rendered timeline.
#[derive(Clone, PartialEq, Eq, Debug)]
pub struct SubtitleCue {
    /// 1-based cue index (SRT numbering).
    pub index: usize,
    /// Cue timing on the rendered output timeline.
    pub rendered: TimeRange,
    /// Caption lines, each within [`SubtitleOptions::max_chars_per_line`]
    /// where possible.
    pub lines: Vec<String>,
}

/// Packs the transcript's rendered timeline into caption cues.
///
/// A cue is flushed when adding the next word would exceed the character
/// budget ([`SubtitleOptions::max_lines`] × [`SubtitleOptions::
/// max_chars_per_line`]) or the duration limit, or when the next word
/// belongs to a different segment.
#[must_use]
pub fn build_cues(
    transcript: &Transcript,
    speakers: &[Speaker],
    options: &SubtitleOptions,
) -> Vec<SubtitleCue> {
    /// Cue under construction; words accumulate until a limit is reached.
    struct Accumulating {
        segment: SegmentId,
        start: Duration,
        end: Duration,
        words: Vec<String>,
        begins_segment: bool,
    }

    let speaker_of: HashMap<SegmentId, Option<SpeakerId>> = transcript
        .segments
        .iter()
        .map(|s| (s.id, s.speaker))
        .collect();
    let label_of = |speaker: Option<SpeakerId>| -> Option<&str> {
        speaker.and_then(|id| {
            speakers
                .iter()
                .find(|s| s.id == id)
                .map(|s| s.label.as_str())
        })
    };

    let char_budget = options.max_lines * options.max_chars_per_line;
    let mut cues: Vec<SubtitleCue> = Vec::new();

    let mut current: Option<Accumulating> = None;

    let flush = |acc: Option<Accumulating>, cues: &mut Vec<SubtitleCue>| {
        let Some(acc) = acc else { return };
        if acc.words.is_empty() {
            return;
        }
        let mut lines = split_lines(&acc.words, options.max_lines, options.max_chars_per_line);
        if options.include_speaker_labels && acc.begins_segment {
            if let Some(label) = label_of(speaker_of.get(&acc.segment).copied().flatten()) {
                lines[0] = format!("{label}: {}", lines[0]);
            }
        }
        let index = cues.len() + 1;
        cues.push(SubtitleCue {
            index,
            rendered: TimeRange {
                start: acc.start,
                end: acc.end,
            },
            lines,
        });
    };

    for entry in render_timeline(transcript) {
        let fits = match &current {
            None => false,
            Some(acc) => {
                acc.segment == entry.segment
                    && entry.rendered.end.saturating_sub(acc.start) <= options.max_cue_duration
                    && total_chars(&acc.words) + 1 + entry.word.text.len() <= char_budget
            }
        };
        if !fits {
            flush(current.take(), &mut cues);
            current = Some(Accumulating {
                segment: entry.segment,
                start: entry.rendered.start,
                end: entry.rendered.end,
                words: vec![entry.word.text.clone()],
                begins_segment: entry.word_index == 0,
            });
        } else if let Some(acc) = current.as_mut() {
            acc.end = entry.rendered.end;
            acc.words.push(entry.word.text.clone());
        }
    }
    flush(current.take(), &mut cues);
    cues
}

/// Renders subtitles in the configured format.
#[must_use]
pub fn render_subtitles(
    transcript: &Transcript,
    speakers: &[Speaker],
    options: &SubtitleOptions,
) -> String {
    let cues = build_cues(transcript, speakers, options);
    let decimal_sep = match options.format {
        SubtitleFormat::Srt => ',',
        SubtitleFormat::Vtt => '.',
    };
    let mut out = String::new();
    if options.format == SubtitleFormat::Vtt {
        out.push_str("WEBVTT\n\n");
    }
    for cue in &cues {
        if options.format == SubtitleFormat::Srt {
            out.push_str(&cue.index.to_string());
            out.push('\n');
        }
        out.push_str(&format_timestamp(cue.rendered.start, decimal_sep));
        out.push_str(" --> ");
        out.push_str(&format_timestamp(cue.rendered.end, decimal_sep));
        out.push('\n');
        for line in &cue.lines {
            out.push_str(line);
            out.push('\n');
        }
        out.push('\n');
    }
    out
}

fn total_chars(words: &[String]) -> usize {
    words.iter().map(String::len).sum::<usize>() + words.len().saturating_sub(1)
}

/// Greedy line packing: fill each line up to the limit, overflow allowed on
/// a single over-long word (visible but honest; better than splitting words).
fn split_lines(words: &[String], max_lines: usize, max_chars: usize) -> Vec<String> {
    let mut lines: Vec<String> = vec![String::new()];
    for word in words {
        let starts_new_line = {
            let line = lines.last().expect("always one line");
            !line.is_empty() && line.len() + 1 + word.len() > max_chars
        };
        if starts_new_line && lines.len() < max_lines {
            lines.push(word.clone());
        } else {
            let line = lines.last_mut().expect("always one line");
            if line.is_empty() {
                line.push_str(word);
            } else {
                line.push(' ');
                line.push_str(word);
            }
        }
    }
    lines
}

/// `HH:MM:SS<sep>mmm` (SRT uses `,`, VTT uses `.`).
fn format_timestamp(t: Duration, decimal_sep: char) -> String {
    let millis = t.subsec_millis();
    let total_secs = t.as_secs();
    let (hours, rem) = (total_secs / 3600, total_secs % 3600);
    let (minutes, seconds) = (rem / 60, rem % 60);
    format!("{hours:02}:{minutes:02}:{seconds:02}{decimal_sep}{millis:03}")
}

#[cfg(test)]
mod tests {
    use std::time::Duration;

    use tpt_app_voice_studio_core::confidence::Confidence;
    use tpt_app_voice_studio_core::id::{RecordingId, SegmentId, SpeakerId};
    use tpt_app_voice_studio_model::export::SubtitleFormat;
    use tpt_app_voice_studio_model::speaker::Speaker;
    use tpt_app_voice_studio_model::transcript::{Segment, Transcript, Word};

    use super::*;

    fn word(text: &str, start_ms: u64, end_ms: u64) -> Word {
        Word {
            text: text.to_string(),
            start: Duration::from_millis(start_ms),
            end: Duration::from_millis(end_ms),
            confidence: Confidence::new(0.9).expect("valid"),
            source_recording: RecordingId::new(1),
        }
    }

    /// Two segments; rendered timeline is gap-less so cue timings are the
    /// concatenated word durations.
    fn transcript() -> Transcript {
        Transcript::from_segments(vec![
            Segment {
                id: SegmentId::new(1),
                speaker: Some(SpeakerId::new(1)),
                words: vec![
                    word("Welcome", 0, 1_000),
                    word("back", 1_000, 2_000),
                    word("to", 2_000, 2_500),
                    word("the", 2_500, 3_000),
                    word("show", 3_000, 4_000),
                ],
            },
            Segment {
                id: SegmentId::new(2),
                speaker: Some(SpeakerId::new(2)),
                words: vec![
                    word("Thanks", 4_000, 5_000),
                    word("for", 5_000, 5_500),
                    word("having", 5_500, 6_500),
                    word("me", 6_500, 7_000),
                ],
            },
        ])
    }

    fn speakers() -> Vec<Speaker> {
        vec![
            Speaker::new(SpeakerId::new(1), "Host"),
            Speaker::new(SpeakerId::new(2), "Guest"),
        ]
    }

    #[test]
    fn srt_output_matches_expected_layout() {
        let options = SubtitleOptions {
            format: SubtitleFormat::Srt,
            max_chars_per_line: 24,
            max_lines: 1,
            ..SubtitleOptions::default()
        };
        let srt = render_subtitles(&transcript(), &speakers(), &options);
        let expected = "1\n00:00:00,000 --> 00:00:04,000\nWelcome back to the show\n\n\
                        2\n00:00:04,000 --> 00:00:07,000\nThanks for having me\n\n";
        assert_eq!(srt, expected);
    }

    #[test]
    fn vtt_uses_dots_and_header() {
        let options = SubtitleOptions {
            format: SubtitleFormat::Vtt,
            ..SubtitleOptions::default()
        };
        let vtt = render_subtitles(&transcript(), &speakers(), &options);
        assert!(vtt.starts_with("WEBVTT\n\n"));
        assert!(vtt.contains("00:00:00.000 --> 00:00:04.000"));
        assert!(!vtt.contains(','), "VTT timestamps use dots");
    }

    #[test]
    fn cues_break_at_duration_limit_and_segment_boundaries() {
        let options = SubtitleOptions {
            max_cue_duration: Duration::from_secs(3),
            max_chars_per_line: 24,
            max_lines: 1,
            ..SubtitleOptions::default()
        };
        let cues = build_cues(&transcript(), &speakers(), &options);
        // Segment 1 splits into cues of ≤3s; segment 2 never joins segment 1.
        assert_eq!(cues.len(), 3);
        assert_eq!(cues[0].rendered.end, Duration::from_secs(3));
        assert_eq!(cues[1].lines[0], "show");
        assert_eq!(cues[2].lines[0], "Thanks for having me");
    }

    #[test]
    fn speaker_labels_prefix_segment_starting_cues() {
        let options = SubtitleOptions {
            include_speaker_labels: true,
            max_chars_per_line: 40,
            max_lines: 2,
            ..SubtitleOptions::default()
        };
        let cues = build_cues(&transcript(), &speakers(), &options);
        assert!(cues[0].lines[0].starts_with("Host: Welcome"));
        assert!(cues[cues.len() - 1].lines[0].starts_with("Guest: Thanks"));
    }

    #[test]
    fn unknown_speaker_is_left_unlabeled() {
        let options = SubtitleOptions {
            include_speaker_labels: true,
            ..SubtitleOptions::default()
        };
        // No registry entries: cues still render, without labels.
        let cues = build_cues(&transcript(), &[], &options);
        assert!(!cues[0].lines[0].contains(':'));
    }

    #[test]
    fn empty_transcript_renders_empty_output() {
        let options = SubtitleOptions::default();
        assert_eq!(render_subtitles(&Transcript::default(), &[], &options), "");
        let vtt_options = SubtitleOptions {
            format: SubtitleFormat::Vtt,
            ..SubtitleOptions::default()
        };
        assert_eq!(
            render_subtitles(&Transcript::default(), &[], &vtt_options),
            "WEBVTT\n\n"
        );
    }

    #[test]
    fn timestamp_formats_use_comma_or_dot() {
        let t = Duration::from_millis(3_601_234);
        assert_eq!(format_timestamp(t, ','), "01:00:01,234");
        assert_eq!(format_timestamp(t, '.'), "01:00:01.234");
        assert_eq!(format_timestamp(Duration::ZERO, ','), "00:00:00,000");
    }
}

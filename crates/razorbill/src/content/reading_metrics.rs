use serde::{Deserialize, Serialize};
use unicode_segmentation::UnicodeSegmentation;

/// The reading speed of an average adult in words per minute (WPM).
///
/// [Source](https://scholarwithin.com/average-reading-speed)
pub const AVERAGE_ADULT_WPM: usize = 238;

/// The number of words in a piece of content.
#[derive(Debug, PartialEq, Eq, PartialOrd, Ord, Hash, Clone, Copy, Serialize, Deserialize)]
pub struct WordCount(pub usize);

/// The number of minutes it would take to read a piece of content.
#[derive(Debug, PartialEq, Eq, PartialOrd, Ord, Hash, Clone, Copy, Serialize, Deserialize)]
pub struct ReadTime(pub usize);

/// The reading metrics for a piece of content.
#[derive(Debug, PartialEq, Eq, PartialOrd, Ord, Clone, Copy)]
pub struct ReadingMetrics {
    pub word_count: WordCount,
    pub read_time: ReadTime,
}

impl ReadingMetrics {
    /// Returns the [`ReadingMetrics`] for the given content, assuming it is read
    /// at the specified words per minute (WPM).
    pub fn for_content(content: &str, wpm: usize) -> Self {
        let word_count = content.unicode_words().count();

        let minimum_words_to_read = wpm - 1;
        let read_time = (word_count + minimum_words_to_read) / wpm;

        Self {
            word_count: WordCount(word_count),
            read_time: ReadTime(read_time),
        }
    }
}

#[cfg(test)]
mod tests {
    use pretty_assertions::assert_eq;

    use super::*;

    #[test]
    fn test_read_time_for_zero_words() {
        let metrics = ReadingMetrics::for_content("", AVERAGE_ADULT_WPM);

        assert_eq!(
            metrics,
            ReadingMetrics {
                word_count: WordCount(0),
                read_time: ReadTime(0)
            }
        );
    }

    #[test]
    fn test_read_time_with_a_single_word() {
        let metrics = ReadingMetrics::for_content("The", AVERAGE_ADULT_WPM);

        assert_eq!(
            metrics,
            ReadingMetrics {
                word_count: WordCount(1),
                read_time: ReadTime(1)
            }
        );
    }
}

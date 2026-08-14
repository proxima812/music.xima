//! Listening statistics (CONTRACTS §1.6).

use serde::{Deserialize, Serialize};

use crate::domain::track::Track;

pub const MS_PER_DAY: i64 = 86_400_000;

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum StatsRange {
    Today,
    Week,
    Month,
    Year,
    #[default]
    AllTime,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct RankedTrack {
    pub track: Track,
    pub plays: i64,
    pub listening_time_ms: i64,
}

impl StatsRange {
    /// Lower bound (inclusive, Unix ms) of `history.played_at` for this range,
    /// `None` for `AllTime`.
    ///
    /// `Today` is the current UTC day; the other ranges are rolling windows
    /// counted back from `now_ms`.
    pub const fn since_ms(self, now_ms: i64) -> Option<i64> {
        match self {
            Self::Today => Some(now_ms.saturating_sub(now_ms.rem_euclid(MS_PER_DAY))),
            Self::Week => Some(now_ms.saturating_sub(7 * MS_PER_DAY)),
            Self::Month => Some(now_ms.saturating_sub(30 * MS_PER_DAY)),
            Self::Year => Some(now_ms.saturating_sub(365 * MS_PER_DAY)),
            Self::AllTime => None,
        }
    }

    /// Whether a timestamp falls into the range. Mirrors the filter the SQL
    /// layer applies, and keeps it testable without a database.
    pub const fn contains(self, timestamp_ms: i64, now_ms: i64) -> bool {
        match self.since_ms(now_ms) {
            Some(since) => timestamp_ms >= since,
            None => true,
        }
    }

    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Today => "TODAY",
            Self::Week => "WEEK",
            Self::Month => "MONTH",
            Self::Year => "YEAR",
            Self::AllTime => "ALL_TIME",
        }
    }

    pub fn from_token(token: &str) -> Option<Self> {
        [
            Self::Today,
            Self::Week,
            Self::Month,
            Self::Year,
            Self::AllTime,
        ]
        .into_iter()
        .find(|range| range.as_str() == token)
    }
}

#[cfg(test)]
mod tests {
    use super::{StatsRange, MS_PER_DAY};

    /// 2023-11-14T22:13:20Z, deliberately not midnight.
    const NOW: i64 = 1_700_000_000_000;

    #[test]
    fn all_time_has_no_lower_bound() {
        assert_eq!(StatsRange::AllTime.since_ms(NOW), None);
        assert!(StatsRange::AllTime.contains(0, NOW));
        assert!(StatsRange::AllTime.contains(i64::MIN, NOW));
    }

    #[test]
    fn today_starts_at_utc_midnight() {
        let since = StatsRange::Today.since_ms(NOW).expect("today is bounded");
        assert_eq!(since, 1_699_920_000_000);
        assert_eq!(since % MS_PER_DAY, 0);
        assert!(since <= NOW);
        assert!(NOW - since < MS_PER_DAY);
    }

    #[test]
    fn midnight_is_its_own_start() {
        let midnight = 1_699_920_000_000;
        assert_eq!(StatsRange::Today.since_ms(midnight), Some(midnight));
    }

    #[test]
    fn rolling_windows() {
        assert_eq!(StatsRange::Week.since_ms(NOW), Some(NOW - 7 * MS_PER_DAY));
        assert_eq!(StatsRange::Month.since_ms(NOW), Some(NOW - 30 * MS_PER_DAY));
        assert_eq!(StatsRange::Year.since_ms(NOW), Some(NOW - 365 * MS_PER_DAY));
    }

    #[test]
    fn contains_respects_bounds() {
        let week_start = NOW - 7 * MS_PER_DAY;
        assert!(StatsRange::Week.contains(week_start, NOW));
        assert!(StatsRange::Week.contains(NOW, NOW));
        assert!(!StatsRange::Week.contains(week_start - 1, NOW));
        assert!(!StatsRange::Today.contains(NOW - MS_PER_DAY, NOW));
    }

    #[test]
    fn pre_epoch_timestamps_do_not_panic() {
        let before_epoch = -1;
        assert_eq!(StatsRange::Today.since_ms(before_epoch), Some(-MS_PER_DAY));
        assert_eq!(StatsRange::Week.since_ms(i64::MIN), Some(i64::MIN));
    }

    #[test]
    fn tokens_match_json() {
        for range in [
            StatsRange::Today,
            StatsRange::Week,
            StatsRange::Month,
            StatsRange::Year,
            StatsRange::AllTime,
        ] {
            let json = serde_json::to_string(&range).expect("range serializes");
            assert_eq!(json, format!("\"{}\"", range.as_str()));
            assert_eq!(StatsRange::from_token(range.as_str()), Some(range));
        }
        assert_eq!(StatsRange::from_token("ALLTIME"), None);
    }
}

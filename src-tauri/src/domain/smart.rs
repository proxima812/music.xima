//! Smart playlists: rules are data, compiled to SQL by the repository layer
//! (CONTRACTS §1.4). No preset ever gets its own method.

use serde::{Deserialize, Serialize};

use crate::domain::playlist::sanitize_playlist_name;
use crate::domain::query::TrackSort;
use crate::error::{CoreError, CoreResult};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum NumOp {
    Eq,
    Ne,
    Lt,
    Lte,
    Gt,
    Gte,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "SCREAMING_SNAKE_CASE")]
pub enum SmartRule {
    #[serde(rename_all = "camelCase")]
    PlayCount {
        op: NumOp,
        value: i64,
    },
    NeverPlayed,
    #[serde(rename_all = "camelCase")]
    LastPlayedBeforeDays {
        days: i64,
    },
    #[serde(rename_all = "camelCase")]
    AddedWithinDays {
        days: i64,
    },
    #[serde(rename_all = "camelCase")]
    Favorite {
        value: bool,
    },
    #[serde(rename_all = "camelCase")]
    YearBetween {
        from: i32,
        to: i32,
    },
    #[serde(rename_all = "camelCase")]
    BitrateAtLeast {
        value: i32,
    },
    #[serde(rename_all = "camelCase")]
    DurationBetweenMs {
        from: i64,
        to: i64,
    },
    #[serde(rename_all = "camelCase")]
    GenreIs {
        value: String,
    },
    #[serde(rename_all = "camelCase")]
    ArtistIs {
        artist_id: i64,
    },
    #[serde(rename_all = "camelCase")]
    TitleContains {
        value: String,
    },
}

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum SmartSort {
    #[default]
    TitleAsc,
    DateAddedDesc,
    PlayCountDesc,
    LastPlayedDesc,
    YearDesc,
    Random,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SmartPlaylist {
    pub id: i64,
    pub name: String,
    pub rules: Vec<SmartRule>,
    /// true = AND between rules, false = OR.
    pub match_all: bool,
    pub sort: SmartSort,
    pub limit: Option<i64>,
    pub created_at: i64,
    pub updated_at: i64,
}

/// Payload of `smart_playlist_create` / `smart_playlist_update` /
/// `smart_playlist_preview` (CONTRACTS §5). Lives in the domain because the
/// repository trait takes it.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SmartPlaylistDraft {
    pub name: String,
    pub rules: Vec<SmartRule>,
    pub match_all: bool,
    pub sort: SmartSort,
    pub limit: Option<i64>,
}

impl NumOp {
    /// In-memory counterpart of the comparison the SQL layer emits.
    pub const fn matches(self, left: i64, right: i64) -> bool {
        match self {
            Self::Eq => left == right,
            Self::Ne => left != right,
            Self::Lt => left < right,
            Self::Lte => left <= right,
            Self::Gt => left > right,
            Self::Gte => left >= right,
        }
    }
}

impl SmartSort {
    /// Stable token, identical to the JSON representation. Stored in
    /// `smart_playlists.sort`.
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::TitleAsc => "TITLE_ASC",
            Self::DateAddedDesc => "DATE_ADDED_DESC",
            Self::PlayCountDesc => "PLAY_COUNT_DESC",
            Self::LastPlayedDesc => "LAST_PLAYED_DESC",
            Self::YearDesc => "YEAR_DESC",
            Self::Random => "RANDOM",
        }
    }

    pub fn from_token(token: &str) -> Option<Self> {
        [
            Self::TitleAsc,
            Self::DateAddedDesc,
            Self::PlayCountDesc,
            Self::LastPlayedDesc,
            Self::YearDesc,
            Self::Random,
        ]
        .into_iter()
        .find(|sort| sort.as_str() == token)
    }

    pub const fn is_random(self) -> bool {
        matches!(self, Self::Random)
    }

    /// Same ordering expressed as a library sort; `None` for `Random`, which
    /// has no deterministic column.
    pub const fn as_track_sort(self) -> Option<TrackSort> {
        match self {
            Self::TitleAsc => Some(TrackSort::TitleAsc),
            Self::DateAddedDesc => Some(TrackSort::DateAddedDesc),
            Self::PlayCountDesc => Some(TrackSort::PlayCountDesc),
            Self::LastPlayedDesc => Some(TrackSort::LastPlayedDesc),
            Self::YearDesc => Some(TrackSort::YearDesc),
            Self::Random => None,
        }
    }
}

impl SmartRule {
    /// Rejects rules that would compile into a query that can never match, or
    /// into a query built from blank user input.
    pub fn validate(&self) -> CoreResult<()> {
        match self {
            Self::PlayCount { value, .. } => {
                require(*value >= 0, "playCount value must not be negative")
            }
            Self::LastPlayedBeforeDays { days } => require(*days > 0, "days must be positive"),
            Self::AddedWithinDays { days } => require(*days > 0, "days must be positive"),
            Self::YearBetween { from, to } => require(from <= to, "yearBetween: from is after to"),
            Self::BitrateAtLeast { value } => require(*value >= 0, "bitrate must not be negative"),
            Self::DurationBetweenMs { from, to } => require(
                *from >= 0 && from <= to,
                "durationBetweenMs: expected 0 <= from <= to",
            ),
            Self::GenreIs { value } => require(!value.trim().is_empty(), "genre must not be blank"),
            Self::ArtistIs { artist_id } => require(*artist_id > 0, "artistId must be positive"),
            Self::TitleContains { value } => {
                require(!value.trim().is_empty(), "title query must not be blank")
            }
            Self::NeverPlayed | Self::Favorite { .. } => Ok(()),
        }
    }

    /// Whether the rule needs `play_counts` / `history` data to be evaluated.
    pub fn needs_play_stats(&self) -> bool {
        matches!(
            self,
            Self::PlayCount { .. } | Self::NeverPlayed | Self::LastPlayedBeforeDays { .. }
        )
    }
}

impl SmartPlaylistDraft {
    /// Returns the draft with the name trimmed, after checking every rule.
    pub fn validated(&self) -> CoreResult<Self> {
        let name = sanitize_playlist_name(&self.name)?;
        Ok(Self {
            name,
            ..self.validated_rules()?
        })
    }

    /// Everything except the name: rules and limit.
    ///
    /// A preview saves nothing, so demanding a name while the user is still
    /// assembling the rules only produces an error where a track list belongs.
    pub fn validated_rules(&self) -> CoreResult<Self> {
        for rule in &self.rules {
            rule.validate()?;
        }
        if let Some(limit) = self.limit {
            require(limit > 0, "limit must be positive")?;
        }
        Ok(Self {
            name: self.name.clone(),
            rules: self.rules.clone(),
            match_all: self.match_all,
            sort: self.sort,
            limit: self.limit,
        })
    }
}

impl SmartPlaylist {
    pub fn as_draft(&self) -> SmartPlaylistDraft {
        SmartPlaylistDraft {
            name: self.name.clone(),
            rules: self.rules.clone(),
            match_all: self.match_all,
            sort: self.sort,
            limit: self.limit,
        }
    }
}

fn require(condition: bool, message: &str) -> CoreResult<()> {
    if condition {
        Ok(())
    } else {
        Err(CoreError::InvalidInput(message.to_owned()))
    }
}

#[cfg(test)]
mod tests {
    use super::{NumOp, SmartPlaylistDraft, SmartRule, SmartSort};
    use crate::domain::query::TrackSort;
    use serde_json::json;

    fn draft(rules: Vec<SmartRule>) -> SmartPlaylistDraft {
        SmartPlaylistDraft {
            name: "  Forgotten  ".to_owned(),
            rules,
            match_all: true,
            sort: SmartSort::LastPlayedDesc,
            limit: Some(50),
        }
    }

    #[test]
    fn rules_are_tagged_by_kind() {
        let value = serde_json::to_value(SmartRule::PlayCount {
            op: NumOp::Gte,
            value: 10,
        })
        .expect("rule serializes");
        assert_eq!(
            value,
            json!({ "kind": "PLAY_COUNT", "op": "GTE", "value": 10 })
        );

        let value = serde_json::to_value(SmartRule::NeverPlayed).expect("rule serializes");
        assert_eq!(value, json!({ "kind": "NEVER_PLAYED" }));

        let value =
            serde_json::to_value(SmartRule::ArtistIs { artist_id: 7 }).expect("rule serializes");
        assert_eq!(value, json!({ "kind": "ARTIST_IS", "artistId": 7 }));

        let value = serde_json::to_value(SmartRule::DurationBetweenMs {
            from: 0,
            to: 120_000,
        })
        .expect("rule serializes");
        assert_eq!(
            value,
            json!({ "kind": "DURATION_BETWEEN_MS", "from": 0, "to": 120_000 })
        );
    }

    #[test]
    fn rules_round_trip() {
        let rules = vec![
            SmartRule::NeverPlayed,
            SmartRule::Favorite { value: true },
            SmartRule::YearBetween {
                from: 2020,
                to: 2029,
            },
            SmartRule::TitleContains {
                value: "live".to_owned(),
            },
        ];
        let json = serde_json::to_string(&rules).expect("rules serialize");
        let back: Vec<SmartRule> = serde_json::from_str(&json).expect("rules deserialize");
        assert_eq!(back, rules);
    }

    #[test]
    fn num_op_compares() {
        assert!(NumOp::Gte.matches(10, 10));
        assert!(!NumOp::Gt.matches(10, 10));
        assert!(NumOp::Ne.matches(1, 2));
        assert!(NumOp::Lte.matches(1, 2));
        assert!(NumOp::Lt.matches(1, 2));
        assert!(NumOp::Eq.matches(2, 2));
    }

    #[test]
    fn smart_sort_tokens_match_json() {
        for sort in [
            SmartSort::TitleAsc,
            SmartSort::DateAddedDesc,
            SmartSort::PlayCountDesc,
            SmartSort::LastPlayedDesc,
            SmartSort::YearDesc,
            SmartSort::Random,
        ] {
            let json = serde_json::to_string(&sort).expect("sort serializes");
            assert_eq!(json, format!("\"{}\"", sort.as_str()));
            assert_eq!(SmartSort::from_token(sort.as_str()), Some(sort));
        }
        assert_eq!(SmartSort::from_token("TITLE_DESC"), None);
    }

    #[test]
    fn smart_sort_maps_to_track_sort_except_random() {
        assert_eq!(
            SmartSort::DateAddedDesc.as_track_sort(),
            Some(TrackSort::DateAddedDesc)
        );
        assert_eq!(SmartSort::Random.as_track_sort(), None);
        assert!(SmartSort::Random.is_random());
        assert!(!SmartSort::TitleAsc.is_random());
    }

    #[test]
    fn valid_rules_pass() {
        let rules = vec![
            SmartRule::PlayCount {
                op: NumOp::Gte,
                value: 0,
            },
            SmartRule::AddedWithinDays { days: 30 },
            SmartRule::YearBetween {
                from: 2020,
                to: 2020,
            },
            SmartRule::DurationBetweenMs { from: 0, to: 1 },
            SmartRule::GenreIs {
                value: "Rock".to_owned(),
            },
        ];
        for rule in &rules {
            assert!(rule.validate().is_ok(), "{rule:?} should be valid");
        }
        let validated = draft(rules).validated().expect("draft is valid");
        assert_eq!(validated.name, "Forgotten");
    }

    #[test]
    fn invalid_rules_are_rejected() {
        let invalid = vec![
            SmartRule::AddedWithinDays { days: 0 },
            SmartRule::LastPlayedBeforeDays { days: -1 },
            SmartRule::YearBetween {
                from: 2020,
                to: 2019,
            },
            SmartRule::DurationBetweenMs { from: 10, to: 5 },
            SmartRule::DurationBetweenMs { from: -1, to: 5 },
            SmartRule::GenreIs {
                value: "   ".to_owned(),
            },
            SmartRule::TitleContains {
                value: String::new(),
            },
            SmartRule::ArtistIs { artist_id: 0 },
            SmartRule::BitrateAtLeast { value: -1 },
            SmartRule::PlayCount {
                op: NumOp::Eq,
                value: -3,
            },
        ];
        for rule in invalid {
            assert!(rule.validate().is_err(), "{rule:?} should be rejected");
        }
    }

    #[test]
    fn draft_rejects_blank_name_and_bad_limit() {
        let mut blank = draft(vec![]);
        blank.name = "  ".to_owned();
        assert!(blank.validated().is_err());

        let mut zero_limit = draft(vec![]);
        zero_limit.limit = Some(0);
        assert!(zero_limit.validated().is_err());

        let mut no_limit = draft(vec![]);
        no_limit.limit = None;
        assert!(no_limit.validated().is_ok());
    }

    #[test]
    fn play_stats_dependency() {
        assert!(SmartRule::NeverPlayed.needs_play_stats());
        assert!(SmartRule::LastPlayedBeforeDays { days: 60 }.needs_play_stats());
        assert!(!SmartRule::Favorite { value: true }.needs_play_stats());
    }
}

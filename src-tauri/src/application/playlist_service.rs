//! Manual and smart playlists (CONTRACTS §5, `playlist_*` / `smart_playlist_*`).
//!
//! Every name is sanitized and every rule set is validated here, before it
//! reaches storage: the repository compiles rules into SQL and must never see a
//! contradictory rule.

use std::sync::Arc;

use crate::application::{validated_id, validated_index, validated_track_ids};
use crate::domain::{
    sanitize_playlist_name, Playlist, SmartPlaylist, SmartPlaylistDraft, Track, MAX_PAGE_LIMIT,
};
use crate::error::{CoreError, CoreResult};
use crate::infrastructure::repositories::{PlaylistRepository, SmartPlaylistRepository};

pub struct PlaylistService {
    playlists: Arc<dyn PlaylistRepository>,
    smart: Arc<dyn SmartPlaylistRepository>,
}

impl PlaylistService {
    pub fn new(
        playlists: Arc<dyn PlaylistRepository>,
        smart: Arc<dyn SmartPlaylistRepository>,
    ) -> Self {
        Self { playlists, smart }
    }

    pub async fn list(&self) -> CoreResult<Vec<Playlist>> {
        self.playlists.list().await
    }

    pub async fn get(&self, id: i64) -> CoreResult<Playlist> {
        let id = validated_id("playlist", id)?;
        self.playlists.get(id).await
    }

    pub async fn create(&self, name: &str) -> CoreResult<Playlist> {
        let name = sanitize_playlist_name(name)?;
        self.playlists.create(&name).await
    }

    pub async fn rename(&self, id: i64, name: &str) -> CoreResult<()> {
        let id = validated_id("playlist", id)?;
        let name = sanitize_playlist_name(name)?;
        self.playlists.rename(id, &name).await
    }

    pub async fn delete(&self, id: i64) -> CoreResult<()> {
        let id = validated_id("playlist", id)?;
        self.playlists.delete(id).await
    }

    /// Tracks in playlist order.
    pub async fn tracks(&self, id: i64) -> CoreResult<Vec<Track>> {
        let id = validated_id("playlist", id)?;
        self.playlists.tracks(id).await
    }

    pub async fn add_tracks(&self, id: i64, track_ids: &[i64]) -> CoreResult<()> {
        let id = validated_id("playlist", id)?;
        if track_ids.is_empty() {
            return Err(CoreError::invalid_input("no tracks to add"));
        }
        validated_track_ids(track_ids)?;
        self.playlists.add_tracks(id, track_ids).await
    }

    pub async fn remove_at(&self, id: i64, position: i64) -> CoreResult<()> {
        let id = validated_id("playlist", id)?;
        validated_index("position", position)?;
        self.playlists.remove_at(id, position).await
    }

    /// Moving an item onto itself is a no-op, not an error: drag-and-drop
    /// reports it constantly.
    pub async fn reorder(&self, id: i64, from: i64, to: i64) -> CoreResult<()> {
        let id = validated_id("playlist", id)?;
        validated_index("from", from)?;
        validated_index("to", to)?;
        if from == to {
            return Ok(());
        }
        self.playlists.reorder(id, from, to).await
    }

    pub async fn smart_list(&self) -> CoreResult<Vec<SmartPlaylist>> {
        self.smart.list().await
    }

    pub async fn smart_get(&self, id: i64) -> CoreResult<SmartPlaylist> {
        let id = validated_id("smart playlist", id)?;
        self.smart.get(id).await
    }

    pub async fn smart_create(&self, draft: &SmartPlaylistDraft) -> CoreResult<SmartPlaylist> {
        let draft = validated_draft(draft)?;
        self.smart.create(&draft).await
    }

    pub async fn smart_update(
        &self,
        id: i64,
        draft: &SmartPlaylistDraft,
    ) -> CoreResult<SmartPlaylist> {
        let id = validated_id("smart playlist", id)?;
        let draft = validated_draft(draft)?;
        self.smart.update(id, &draft).await
    }

    pub async fn smart_delete(&self, id: i64) -> CoreResult<()> {
        let id = validated_id("smart playlist", id)?;
        self.smart.delete(id).await
    }

    /// Resolves a stored smart playlist into the tracks it currently matches.
    pub async fn smart_resolve(&self, id: i64) -> CoreResult<Vec<Track>> {
        let id = validated_id("smart playlist", id)?;
        let playlist = self.smart.get(id).await?;
        self.smart
            .resolve(
                &playlist.rules,
                playlist.match_all,
                playlist.sort,
                playlist.limit,
            )
            .await
    }

    /// Resolves an unsaved draft. The result feeds a preview list, so an
    /// unbounded draft is still capped at one page.
    pub async fn smart_preview(&self, draft: &SmartPlaylistDraft) -> CoreResult<Vec<Track>> {
        let draft = previewable_draft(draft)?;
        let limit = draft.limit.unwrap_or(MAX_PAGE_LIMIT).min(MAX_PAGE_LIMIT);
        self.smart
            .resolve(&draft.rules, draft.match_all, draft.sort, Some(limit))
            .await
    }
}

/// A rule-less smart playlist would select the whole library, which is what the
/// plain library screen is for; the domain check covers everything else.
fn validated_draft(draft: &SmartPlaylistDraft) -> CoreResult<SmartPlaylistDraft> {
    require_rules(draft)?;
    draft.validated()
}

/// Same, minus the name: the preview runs while the playlist is still unnamed.
fn previewable_draft(draft: &SmartPlaylistDraft) -> CoreResult<SmartPlaylistDraft> {
    require_rules(draft)?;
    draft.validated_rules()
}

fn require_rules(draft: &SmartPlaylistDraft) -> CoreResult<()> {
    if draft.rules.is_empty() {
        return Err(CoreError::invalid_input(
            "smart playlist needs at least one rule",
        ));
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use std::sync::{Arc, Mutex};

    use super::PlaylistService;
    use crate::application::testing::track;
    use crate::domain::{
        NumOp, Playlist, SmartPlaylist, SmartPlaylistDraft, SmartRule, SmartSort, Track,
        MAX_PAGE_LIMIT, MAX_PLAYLIST_NAME_LEN,
    };
    use crate::error::CoreResult;
    use crate::infrastructure::repositories::{PlaylistRepository, SmartPlaylistRepository};

    #[derive(Default)]
    struct FakePlaylists {
        created: Mutex<Vec<String>>,
        renamed: Mutex<Vec<(i64, String)>>,
        added: Mutex<Vec<(i64, Vec<i64>)>>,
        reordered: Mutex<Vec<(i64, i64, i64)>>,
        removed: Mutex<Vec<(i64, i64)>>,
    }

    fn playlist(id: i64, name: &str) -> Playlist {
        Playlist {
            id,
            name: name.to_owned(),
            track_count: 0,
            duration_ms: 0,
            cover_key: None,
            created_at: 1_700_000_000_000,
            updated_at: 1_700_000_000_000,
        }
    }

    #[async_trait::async_trait]
    impl PlaylistRepository for FakePlaylists {
        async fn list(&self) -> CoreResult<Vec<Playlist>> {
            Ok(vec![playlist(1, "Road trip")])
        }

        async fn get(&self, id: i64) -> CoreResult<Playlist> {
            Ok(playlist(id, "Road trip"))
        }

        async fn create(&self, name: &str) -> CoreResult<Playlist> {
            self.created.lock().expect("lock").push(name.to_owned());
            Ok(playlist(1, name))
        }

        async fn rename(&self, id: i64, name: &str) -> CoreResult<()> {
            self.renamed
                .lock()
                .expect("lock")
                .push((id, name.to_owned()));
            Ok(())
        }

        async fn delete(&self, _id: i64) -> CoreResult<()> {
            Ok(())
        }

        async fn tracks(&self, _id: i64) -> CoreResult<Vec<Track>> {
            Ok(vec![track(1), track(2)])
        }

        async fn add_tracks(&self, id: i64, track_ids: &[i64]) -> CoreResult<()> {
            self.added
                .lock()
                .expect("lock")
                .push((id, track_ids.to_vec()));
            Ok(())
        }

        async fn remove_at(&self, id: i64, position: i64) -> CoreResult<()> {
            self.removed.lock().expect("lock").push((id, position));
            Ok(())
        }

        async fn reorder(&self, id: i64, from: i64, to: i64) -> CoreResult<()> {
            self.reordered.lock().expect("lock").push((id, from, to));
            Ok(())
        }
    }

    /// Arguments a single `resolve` call was made with.
    type ResolveCall = (Vec<SmartRule>, bool, SmartSort, Option<i64>);

    #[derive(Default)]
    struct FakeSmart {
        saved: Mutex<Vec<SmartPlaylistDraft>>,
        resolved: Mutex<Vec<ResolveCall>>,
    }

    fn smart(id: i64) -> SmartPlaylist {
        SmartPlaylist {
            id,
            name: "Forgotten".to_owned(),
            rules: vec![SmartRule::LastPlayedBeforeDays { days: 60 }],
            match_all: true,
            sort: SmartSort::LastPlayedDesc,
            limit: Some(25),
            created_at: 1_700_000_000_000,
            updated_at: 1_700_000_000_000,
        }
    }

    #[async_trait::async_trait]
    impl SmartPlaylistRepository for FakeSmart {
        async fn list(&self) -> CoreResult<Vec<SmartPlaylist>> {
            Ok(vec![smart(1)])
        }

        async fn get(&self, id: i64) -> CoreResult<SmartPlaylist> {
            Ok(smart(id))
        }

        async fn create(&self, draft: &SmartPlaylistDraft) -> CoreResult<SmartPlaylist> {
            self.saved.lock().expect("lock").push(draft.clone());
            let mut created = smart(1);
            created.name = draft.name.clone();
            Ok(created)
        }

        async fn update(&self, id: i64, draft: &SmartPlaylistDraft) -> CoreResult<SmartPlaylist> {
            self.saved.lock().expect("lock").push(draft.clone());
            let mut updated = smart(id);
            updated.name = draft.name.clone();
            Ok(updated)
        }

        async fn delete(&self, _id: i64) -> CoreResult<()> {
            Ok(())
        }

        async fn resolve(
            &self,
            rules: &[SmartRule],
            match_all: bool,
            sort: SmartSort,
            limit: Option<i64>,
        ) -> CoreResult<Vec<Track>> {
            self.resolved
                .lock()
                .expect("lock")
                .push((rules.to_vec(), match_all, sort, limit));
            Ok(vec![track(1)])
        }
    }

    fn service() -> (PlaylistService, Arc<FakePlaylists>, Arc<FakeSmart>) {
        let playlists = Arc::new(FakePlaylists::default());
        let smart_repo = Arc::new(FakeSmart::default());
        (
            PlaylistService::new(playlists.clone(), smart_repo.clone()),
            playlists,
            smart_repo,
        )
    }

    fn draft(rules: Vec<SmartRule>) -> SmartPlaylistDraft {
        SmartPlaylistDraft {
            name: "  Fresh  ".to_owned(),
            rules,
            match_all: true,
            sort: SmartSort::DateAddedDesc,
            limit: Some(30),
        }
    }

    #[tokio::test]
    async fn create_trims_the_name() {
        let (service, playlists, _) = service();

        let created = service.create("  Road trip \n").await.expect("created");

        assert_eq!(created.name, "Road trip");
        assert_eq!(
            playlists.created.lock().expect("lock").as_slice(),
            &["Road trip".to_owned()]
        );
    }

    #[tokio::test]
    async fn create_rejects_blank_and_oversized_names() {
        let (service, playlists, _) = service();

        assert_eq!(
            service.create("   ").await.expect_err("blank name").code(),
            "INVALID_INPUT"
        );
        let long = "n".repeat(MAX_PLAYLIST_NAME_LEN + 1);
        assert_eq!(
            service.create(&long).await.expect_err("long name").code(),
            "INVALID_INPUT"
        );
        assert_eq!(
            service
                .rename(1, "\t")
                .await
                .expect_err("blank name")
                .code(),
            "INVALID_INPUT"
        );
        assert_eq!(
            service.rename(0, "Fine").await.expect_err("bad id").code(),
            "INVALID_INPUT"
        );
        assert!(playlists.created.lock().expect("lock").is_empty());
        assert!(playlists.renamed.lock().expect("lock").is_empty());
    }

    #[tokio::test]
    async fn add_tracks_requires_a_non_empty_positive_list() {
        let (service, playlists, _) = service();

        assert_eq!(
            service.add_tracks(1, &[]).await.expect_err("empty").code(),
            "INVALID_INPUT"
        );
        assert_eq!(
            service
                .add_tracks(1, &[3, 0])
                .await
                .expect_err("zero id")
                .code(),
            "INVALID_INPUT"
        );
        service.add_tracks(1, &[3, 4]).await.expect("added");

        assert_eq!(
            playlists.added.lock().expect("lock").as_slice(),
            &[(1, vec![3, 4])]
        );
    }

    #[tokio::test]
    async fn reorder_skips_no_op_moves_and_rejects_negative_positions() {
        let (service, playlists, _) = service();

        service.reorder(1, 2, 2).await.expect("no-op is fine");
        assert!(playlists.reordered.lock().expect("lock").is_empty());

        assert_eq!(
            service
                .reorder(1, -1, 2)
                .await
                .expect_err("negative")
                .code(),
            "INVALID_INPUT"
        );
        assert_eq!(
            service.remove_at(1, -2).await.expect_err("negative").code(),
            "INVALID_INPUT"
        );

        service.reorder(1, 0, 3).await.expect("reordered");
        assert_eq!(
            playlists.reordered.lock().expect("lock").as_slice(),
            &[(1, 0, 3)]
        );
    }

    #[tokio::test]
    async fn smart_draft_must_carry_at_least_one_valid_rule() {
        let (service, _, smart_repo) = service();

        assert_eq!(
            service
                .smart_create(&draft(Vec::new()))
                .await
                .expect_err("no rules")
                .code(),
            "INVALID_INPUT"
        );
        assert_eq!(
            service
                .smart_create(&draft(vec![SmartRule::YearBetween {
                    from: 2029,
                    to: 2020,
                }]))
                .await
                .expect_err("reversed year range")
                .code(),
            "INVALID_INPUT"
        );
        assert_eq!(
            service
                .smart_create(&draft(vec![SmartRule::DurationBetweenMs {
                    from: 240_000,
                    to: 60_000,
                }]))
                .await
                .expect_err("reversed duration range")
                .code(),
            "INVALID_INPUT"
        );

        let mut zero_limit = draft(vec![SmartRule::Favorite { value: true }]);
        zero_limit.limit = Some(0);
        assert_eq!(
            service
                .smart_create(&zero_limit)
                .await
                .expect_err("zero limit")
                .code(),
            "INVALID_INPUT"
        );

        assert!(smart_repo.saved.lock().expect("lock").is_empty());
    }

    #[tokio::test]
    async fn smart_create_stores_the_sanitized_draft() {
        let (service, _, smart_repo) = service();

        let created = service
            .smart_create(&draft(vec![SmartRule::PlayCount {
                op: NumOp::Gte,
                value: 5,
            }]))
            .await
            .expect("created");

        assert_eq!(created.name, "Fresh");
        let saved = smart_repo.saved.lock().expect("lock").clone();
        assert_eq!(saved.len(), 1);
        assert_eq!(saved[0].name, "Fresh", "name reaches storage trimmed");
        assert_eq!(saved[0].limit, Some(30));
    }

    #[tokio::test]
    async fn smart_resolve_uses_the_stored_definition() {
        let (service, _, smart_repo) = service();

        let tracks = service.smart_resolve(9).await.expect("resolved");

        assert_eq!(tracks.len(), 1);
        let calls = smart_repo.resolved.lock().expect("lock").clone();
        assert_eq!(calls.len(), 1);
        assert_eq!(
            calls[0].0,
            vec![SmartRule::LastPlayedBeforeDays { days: 60 }]
        );
        assert!(calls[0].1);
        assert_eq!(calls[0].2, SmartSort::LastPlayedDesc);
        assert_eq!(calls[0].3, Some(25));
    }

    #[tokio::test]
    async fn smart_preview_caps_an_unbounded_draft() {
        let (service, _, smart_repo) = service();
        let mut unbounded = draft(vec![SmartRule::NeverPlayed]);
        unbounded.limit = None;

        service.smart_preview(&unbounded).await.expect("preview");

        let calls = smart_repo.resolved.lock().expect("lock").clone();
        assert_eq!(calls[0].3, Some(MAX_PAGE_LIMIT));
    }

    #[tokio::test]
    async fn smart_preview_runs_before_the_playlist_is_named() {
        let (service, _, smart_repo) = service();
        let mut unnamed = draft(vec![SmartRule::NeverPlayed]);
        unnamed.name = String::new();

        service.smart_preview(&unnamed).await.expect("preview");

        assert_eq!(smart_repo.resolved.lock().expect("lock").len(), 1);
    }

    #[tokio::test]
    async fn saving_still_demands_a_name() {
        let (service, _, _) = service();
        let mut unnamed = draft(vec![SmartRule::NeverPlayed]);
        unnamed.name = "   ".to_owned();

        assert_eq!(
            service
                .smart_create(&unnamed)
                .await
                .expect_err("name is required")
                .code(),
            "INVALID_INPUT"
        );
    }

    #[tokio::test]
    async fn preview_still_needs_at_least_one_rule() {
        let (service, _, smart_repo) = service();
        let empty = draft(Vec::new());

        assert_eq!(
            service
                .smart_preview(&empty)
                .await
                .expect_err("rules are required")
                .code(),
            "INVALID_INPUT"
        );
        assert!(smart_repo.resolved.lock().expect("lock").is_empty());
    }
}

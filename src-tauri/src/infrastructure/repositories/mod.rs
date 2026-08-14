//! Repository interfaces (CONTRACTS §3).
//!
//! Only traits live here; the SQLite implementations are in
//! `infrastructure::sqlite`. Application services depend on these traits, never
//! on a concrete store.

pub mod album;
pub mod artist;
pub mod favorite;
pub mod history;
pub mod playlist;
pub mod search;
pub mod smart;
pub mod statistics;
pub mod taxonomy;
pub mod track;

pub use album::AlbumRepository;
pub use artist::ArtistRepository;
pub use favorite::FavoriteRepository;
pub use history::HistoryRepository;
pub use playlist::PlaylistRepository;
pub use search::SearchRepository;
pub use smart::SmartPlaylistRepository;
pub use statistics::StatisticsRepository;
pub use taxonomy::TaxonomyRepository;
pub use track::TrackRepository;

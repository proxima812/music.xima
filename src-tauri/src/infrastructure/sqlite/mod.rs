//! SQLite implementations of the repository traits (CONTRACTS §3, §4).
//!
//! This is the only place in the project that contains SQL. Everything above it
//! depends on the traits in `infrastructure::repositories`, never on these
//! structs.

pub mod album_repo;
pub mod artist_repo;
pub mod favorite_repo;
pub mod history_repo;
pub mod playlist_repo;
pub mod pool;
pub mod search_repo;
pub mod smart_repo;
pub mod sql;
pub mod statistics_repo;
pub mod taxonomy_repo;
pub mod track_repo;

pub use album_repo::SqliteAlbumRepository;
pub use artist_repo::SqliteArtistRepository;
pub use favorite_repo::SqliteFavoriteRepository;
pub use history_repo::SqliteHistoryRepository;
pub use playlist_repo::SqlitePlaylistRepository;
pub use pool::{connect, Db, DB_FILE_NAME};
pub use search_repo::SqliteSearchRepository;
pub use smart_repo::SqliteSmartPlaylistRepository;
pub use sql::now_ms;
pub use statistics_repo::SqliteStatisticsRepository;
pub use taxonomy_repo::SqliteTaxonomyRepository;
pub use track_repo::SqliteTrackRepository;

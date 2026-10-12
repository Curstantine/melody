//! Memory benchmark for the Melody core library.
//!
//! Indexes a synthetic library (default 100k tracks with heavy metadata
//! repetition) and reports process RSS plus peak RSS at each stage of the
//! catalog/query surface. Run with:
//!
//! ```bash
//! cargo run -p core --release --example bench_memory
//! BENCH_N=50000 cargo run -p core --release --example bench_memory
//! ```
//!
//! Note: a custom `#[global_allocator]` cannot be installed here because a
//! dependency of `turso` already registers one, so heap accounting is limited
//! to resident-set size, which is the metric this project actually targets.

use core_engine::memory::{format_bytes, peak_resident_bytes, resident_bytes};
use core_engine::{LibraryDatabase, NewTrack, SortDirection, SortField};

fn report(stage: &str) {
    let rss = resident_bytes()
        .map(format_bytes)
        .unwrap_or_else(|| "n/a".into());
    let peak = peak_resident_bytes()
        .map(format_bytes)
        .unwrap_or_else(|| "n/a".into());
    println!("{stage:<34} rss={:>10}  peak_rss={:>10}", rss, peak);
}

fn synth_track(i: usize) -> NewTrack {
    // Heavy but realistic repetition: few thousand artists, more albums.
    let artist = format!("Artist {:04}", i % 5_000);
    let album = format!("Album {:05}", i % 12_500);
    NewTrack {
        file_path: format!("/music/library/track_{:06}.flac", i),
        title: format!("Track Title Number {:06}", i),
        artist: artist.clone(),
        album_artist: Some(artist),
        album,
        genre: Some(["Rock", "Jazz", "Electronic", "Classical"][i % 4].to_string()),
        duration_ms: 120_000 + (i as u64 % 120_000),
        track_num: Some((i % 20) as u32 + 1),
        disc_num: Some(1),
        year: Some(1980 + (i % 45) as u32),
        composer: None,
        label: Some(format!("Label {}", i % 50)),
        musicbrainz_track_id: None,
        musicbrainz_artist_id: None,
        musicbrainz_album_id: None,
    }
}

#[tokio::main(flavor = "current_thread")]
async fn main() {
    let n: usize = std::env::var("BENCH_N")
        .ok()
        .and_then(|v| v.parse().ok())
        .unwrap_or(100_000);

    println!("Melody core memory benchmark — N = {n} tracks");
    println!("{:-<78}", "");
    report("baseline (before indexing)");

    let db = LibraryDatabase::in_memory()
        .await
        .expect("in-memory database failed");

    // ---- Index ----
    let mut batch = Vec::with_capacity(1_000);
    for i in 0..n {
        batch.push(synth_track(i));
        if batch.len() == 1_000 {
            db.insert_tracks_batch(&batch)
                .await
                .expect("batch insert failed");
            batch.clear();
        }
    }
    if !batch.is_empty() {
        db.insert_tracks_batch(&batch)
            .await
            .expect("final batch failed");
    }
    report("after indexing");

    let total = db.get_total_track_count(None).await.unwrap_or(0);
    println!("indexed total = {total}");

    // ---- Virtualized window (the new UI path) ----
    // Measured before loading full catalogs so the delta is attributable.
    for _ in 0..8 {
        let _ = db
            .get_track_rows_window(0, 50, None, SortField::Title, SortDirection::Ascending)
            .await
            .expect("window failed");
    }
    let page = db
        .get_track_rows_window(0, 50, None, SortField::Title, SortDirection::Ascending)
        .await
        .expect("window failed");
    report("after one 50-row virtualized window");
    println!("rows = {}", page.len());
    drop(page);

    // ---- Catalogs ----
    let albums = db.get_albums().await.expect("get_albums failed");
    report("after get_albums()");
    println!("albums = {}", albums.len());
    drop(albums);

    let artists = db.get_artists().await.expect("get_artists failed");
    report("after get_artists()");
    println!("artists = {}", artists.len());
    drop(artists);

    let headers = db.get_all_track_headers().await.expect("headers failed");
    report("after get_all_track_headers()");
    println!(
        "headers = {} ({} bytes each)",
        headers.len(),
        std::mem::size_of_val(&headers[0])
    );
    drop(headers);

    // Legacy behavior: stream pages into a single growing Vec (what the
    // unbounded LibraryView used to do after scrolling to the bottom).
    let mut accumulated = Vec::new();
    let mut offset = 0;
    while offset < total {
        let chunk = db
            .get_tracks_window(
                offset,
                2_000,
                None,
                SortField::Title,
                SortDirection::Ascending,
            )
            .await
            .expect("window failed");
        if chunk.is_empty() {
            break;
        }
        offset += chunk.len();
        accumulated.extend(chunk);
    }
    report("legacy: full library in memory");
    println!("accumulated tracks = {}", accumulated.len());
    drop(accumulated);

    report("after dropping everything");
    println!("{:-<78}", "");
}

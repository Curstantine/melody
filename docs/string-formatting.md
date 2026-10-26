# String Formatting & View Templates

Melody includes a lightweight, flexible template formatting engine in [`core_engine::template`](file:///crates/core/src/template.rs) (accessible via [`format_template`](file:///crates/core/src/template.rs#L112)). It allows users to dynamically customize how tracks, albums, artists, playlists, and queues are displayed throughout the UI via KDL layout files.

---

## 1. Syntax Overview

The templating engine supports three core constructs:

1. **Placeholders (`{field}`)**:
   Substituted with the value of `field` looked up from the current item. If the field is missing, undefined, or empty, it resolves to an empty string `""`.
   ```text
   {title} - {artist}
   ```

2. **Conditional Groups (`[...{field}...]`)**:
   Square brackets enclose optional template fragments. The entire enclosed group is **only rendered if all placeholders within it resolve to non-empty values**. If any field within the group is missing or empty, the entire group inside the brackets is omitted.
   ```text
   {artist}[ • {year_label}]
   ```
   - If `year_label` is `"2025"`, output: `"Imperial Circus Dead Decadence • 2025"`.
   - If `year_label` is empty/missing, output: `"Imperial Circus Dead Decadence"`.

3. **Escaping Literals (`{{` and `}}`)**:
   To render a literal `{` or `}`, escape them by doubling the braces:
   ```text
   {{literal}} -> {literal}
   ```

4. **Automatic Separator & Whitespace Cleanup**:
   The engine automatically cleans up consecutive whitespaces and eliminates dangling punctuation separators (`•`, `-`, `,`, `|`, `/`) caused by missing or empty fields.
   For example, if `{genre}` is missing:
   ```text
   {title} • {genre} -> "Yūaku no Inori - Anima immortalis est."  (dangling • is stripped)
   ```

---

## 2. Supported Fields Reference

Depending on the context / view where the formatting string is used, different fields are available for substitution:

### Track Row (`TrackRow`)
*Used in Library table columns, playlist detail rows, and queue track rows.*

| Field Name | Aliases | Description | Example |
| :--- | :--- | :--- | :--- |
| `{id}` | | Database ID of the track | `101` |
| `{title}` | | Track title | `Yūaku no Inori - Anima immortalis est.` |
| `{artist}` | | Display artist name (falls back to track artist) | `Imperial Circus Dead Decadence` |
| `{album}` | | Album name | `Yūaku no Inori - Anima immortalis est.` |
| `{album_artist}` | | Album artist name (if set) | `Imperial Circus Dead Decadence` |
| `{genre}` | | Genre name (if set) | `Symphonic Melodic Death Metal` |
| `{year}` | | Release year (number) | `2025` |
| `{track_num}` | `{track_number}` | Track number on disc | `1` |
| `{disc_num}` | `{disc_number}` | Disc number | `1` |
| `{duration_ms}` | | Track duration in milliseconds | `472000` |
| `{duration_label}` | `{duration}` | Formatted duration string | `7:52` |
| `{file_path}` | `{path}` | Absolute or relative file path | `/music/ICDD - Anima.flac` |

### Album Summary (`AlbumSummary`)
*Used in Albums view (grid tiles and list rows).*

| Field Name | Aliases | Description | Example |
| :--- | :--- | :--- | :--- |
| `{album}` | `{title}` | Album title | `Yūaku no Inori - Anima immortalis est.` |
| `{artist}` | `{album_artist}` | Album artist name | `Imperial Circus Dead Decadence` |
| `{track_count}` | `{tracks_count}` | Number of tracks in album | `1` |
| `{tracks_label}` | `{tracks}` | Localized track count string | `1 track` |
| `{year}` | | Release year (number) | `2025` |
| `{year_label}` | | Formatted release year | `2025` |
| `{duration_label}` | `{duration}` | Formatted total album duration | `7:52` |
| `{total_duration_ms}` | | Total album duration in milliseconds | `472000` |
| `{cover_track_id}` | | Track ID used for album cover extraction | `101` |
| `{added_timestamp}` | | Unix timestamp when album tracks were added | `1733529600` |

### Artist Summary (`ArtistSummary`)
*Used in Artists view (grid tiles and list rows).*

| Field Name | Aliases | Description | Example |
| :--- | :--- | :--- | :--- |
| `{artist}` | `{name}` | Artist name | `Imperial Circus Dead Decadence` |
| `{track_count}` | `{tracks}` | Total number of tracks by artist | `48` |
| `{album_count}` | `{albums}` | Total number of albums by artist | `5` |
| `{stats_label}` | `{stats}` | Pre-formatted summary stats | `5 albums • 48 tracks` |

### Playlist Summary (`PlaylistSummary`)
*Used in Playlists catalog view (grid tiles and list rows).*

| Field Name | Aliases | Description | Example |
| :--- | :--- | :--- | :--- |
| `{id}` | | Playlist database ID | `42` |
| `{name}` | `{title}` | Playlist name | `Japanese Metal Favorites` |
| `{file_path}` | `{path}` | Path to `.m3u8` playlist file | `/path/japanese_metal.m3u8` |
| `{track_count}` | | Number of tracks in playlist | `24` |
| `{tracks_label}` | `{tracks}` | Localized track count string | `24 tracks` |
| `{managed}` | | Managed status string | `"Managed"` or `"External"` |
| `{created_timestamp}` | | Unix creation timestamp | `1700000000` |
| `{modified_timestamp}`| | Unix last modified timestamp | `1700000000` |

### Queue Record (`QueueRecord`)
*Used in Queues chips and list headers.*

| Field Name | Aliases | Description | Example |
| :--- | :--- | :--- | :--- |
| `{id}` | | Queue database ID | `1` |
| `{name}` | `{title}` | Queue name | `Album: Yūaku no Inori - Anima immortalis est.` |
| `{current}` | | Index of the active track | `0` |
| `{source_kind}` | `{source}` | Queue source kind (`album`, `artist`, etc.) | `album` |
| `{source_value}` | | Specific source identifier | `Yūaku no Inori - Anima immortalis est.` |
| `{source_playlist}` | | Source playlist identifier (if applicable) | `None` |
| `{track_count}` | `{tracks}` | Total tracks in queue | `1` |

---

## 3. Configuring View Formats in KDL Layouts

Formatting strings are declared directly in your KDL layout files (e.g., `layouts/default.kdl` or custom layouts in `~/.config/melody/layouts/`) under the `view-formats` node.

### Example KDL Configuration

```kdl
layout name="Custom Layout" description="Layout with customized view templates" {
    view-formats {
        library {
            column "Track #" format="[{track_num}]" width=60 sort="track_num"
            column "Title" format="{title}" fill-portion=4 sort="title"
            column "Artist" format="{artist}" fill-portion=3 sort="artist"
            column "Album" format="{album}[ ({year})]" fill-portion=3 sort="album"
            column "Duration" format="{duration_label}" width=80 sort="duration"
        }

        albums {
            tile line1="{album}" line2="{artist}[ • {year_label}]" line3="{tracks_label} • {duration_label}"
            list line1="{album}" line2="{artist}[ • {year_label}]" details="{tracks_label} • {duration_label}"
        }

        artists {
            tile line1="{artist}" line2="{stats_label}"
            list line1="{artist}" line2="{albums} albums ({tracks} tracks)"
        }

        playlists {
            tile line1="{name}" line2="{tracks_label}"
            list line1="{name}" line2="{file_path}" details="{tracks_label}"
            detail primary="{title}" secondary="{artist}[ • {album}]" details="{duration_label}"
        }

        queue {
            track primary="{title}" secondary="{artist}[ • {album}]" details="{duration_label}"
            chip format="{name} ({track_count})"
        }
    }

    column {
        navbar height=32
        divider orientation="horizontal"
        main-view fill=true
        player-bar height=72
        status-bar height=24
    }
}
```

### Layout Properties Reference

- **`library`**:
  - `column <title> format="<template>" [fill-portion=<int> | width=<pixels>] [sort="<field>"]`
  - Valid `sort` options: `"title"`, `"artist"`, `"album"`, `"duration"`, `"year"`, `"genre"`, `"track_num"`.
- **`albums`**:
  - `tile`: `line1`, `line2`, `line3` templates for grid tiles.
  - `list`: `line1`, `line2`, `details` templates for list rows.
- **`artists`**:
  - `tile`: `line1`, `line2` templates for grid tiles.
  - `list`: `line1`, `line2` templates for list rows.
- **`playlists`**:
  - `tile`: `line1`, `line2` templates for playlist catalog tiles.
  - `list`: `line1`, `line2`, `details` templates for playlist catalog rows.
  - `detail`: `primary`, `secondary`, `details` templates for track rows in the playlist detail view.
- **`queue`**:
  - `track`: `primary`, `secondary`, `details` templates for track rows in the queue.
  - `chip`: `format` template for the queue selector chip buttons.

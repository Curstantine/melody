# Library Search Syntax

Melody's library search bar supports both plain free-text queries and structured field-scoped filters, parsed by [`TrackFilter::parse`](file:///crates/model/src/search.rs#L33) and evaluated against the SQLite library in [`filter_clause`](file:///crates/db/src/lib.rs#L1462).

An empty query matches the entire library.


## 1. Free-Text Search

Any term without a recognized `field:` prefix is treated as a free-text search term.

- **Matching**: Substring match (`LIKE %term%`) across **`title`**, **`artist`**, or **`album`**.
- **Multiple terms**: Space-separated terms are combined with `AND` — every term must match at least one of those three columns.

```text
miles blue
```
Matches tracks where `miles` appears in `title`, `artist`, or `album` **AND** `blue` appears in `title`, `artist`, or `album`.


## 2. Field-Scoped Filters

Use `field:value` to restrict a term to a specific metadata column. Field names are case-insensitive.

| Field | Alias | Match Type | Example |
| :--- | :--- | :--- | :--- |
| `title` | — | Substring (`LIKE %value%`) | `title:so` |
| `album` | — | Exact (`=`) | `album:"Kind of Blue"` |
| `artist` | — | Exact (`=`) | `artist:"Miles Davis"` |
| `album_artist` | `albumartist` | Exact (`=`) | `album_artist:"Various Artists"` |
| `genre` | — | Exact (`=`) | `genre:Jazz` |
| `year` | — | Exact integer (`=`) | `year:1959` |
| `composer` | — | Exact (`=`) | `composer:"Johann Sebastian Bach"` |
| `label` | — | Exact (`=`) | `label:Columbia` |
| `track` | `track_num` | Exact integer (`=`) | `track:1` |
| `disc` | `disc_num` | Exact integer (`=`) | `disc:2` |

> [!NOTE]
> Categorical fields (`album`, `artist`, `album_artist`, `genre`, `year`) use **exact** matching (`=`) rather than substring matching. This ensures that clicking an album or artist in the catalog filters to *only* that exact item rather than every track that happens to contain its name.

All specified fields and free-text terms are combined with `AND`:

```text
artist:"Miles Davis" genre:Jazz year:1959
```


## 3. Quoting & Escaping

Queries are split into tokens on whitespace.

- **Spaces in values**: Wrap the value in double quotes (`"`):
  ```text
  album:"Kind of Blue"
  ```
- **Escaping inside quotes**: Use a backslash (`\`) to escape a literal double quote (`\"`) or backslash (`\\`) inside a quoted string:
  ```text
  title:"\"Live\" at the Vanguard"
  ```


## 4. Edge Cases & Fallback Behavior

- **Unknown field prefixes**: If `key:value` uses an unrecognized field name (e.g., `composer:Bach`), the entire token is treated as a free-text search term.
- **Invalid integer fields (`year`, `track`, `disc`)**: If `year:`, `track:`, or `disc:` is not a valid unsigned integer (e.g., `year:nineties`), the token falls back to free-text search.
- **Empty values**: A field with an empty or whitespace-only value (e.g., `album:` or `artist:""`) is ignored.
- **Repeated fields**: If the same field appears multiple times in one query, the later value overwrites the earlier one.


## 5. Playback Queues from Search

When you start playback from a filtered library view, Melody names the resulting queue after the dominant scope in the filter:

1. `album` (highest priority)
2. `artist`
3. `album_artist`
4. `genre`
5. `composer`
6. `label`
7. Free-text terms (the single term if there is one, or `Search: <terms>` if there are multiple)

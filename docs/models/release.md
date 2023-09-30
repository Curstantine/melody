# models/Release

Refers to a release done by an artist. A release can be an album, a single, an EP, a compilation, a live recording, etc.

- [models/Release](#modelsrelease)
  - [Properties](#properties)
    - [Notes](#notes)
  - [Shared Types](#shared-types)
    - [ReleaseType](#releasetype)
    - [ReleaseTypeSecondary](#releasetypesecondary)
    - [CountryCode](#countrycode)
    - [ScriptCode](#scriptcode)
    - [ReleaseArtist](#releaseartist)
  - [Pitfalls](#pitfalls)
  - [Examples](#examples)
    - [`artists` with `ReleaseArtist`](#artists-with-releaseartist)

## Properties

| Name           | Type                                              | Description                          | Required |
| -------------- | ------------------------------------------------- | ------------------------------------ | -------- |
| id             | `string`                                          | The ULID of the release.             | Yes      |
| title          | `string`                                          | The title of the release.            | Yes      |
| title_sort     | `string`                                          | The sorting title of the release.    | No       |
| type           | [`ReleaseType`](#releasetype)                     | The primary type of the release.     | Yes      |
| type_secondary | [`ReleaseTypeSecondary[]`](#releasetypesecondary) | The secondary types of the release.  | No       |
| artist         | `string`                                          | The display name of the artists.     | No       |
| artist_id      | `string`                                          | The ULID of an artist.               | No       |
| artist_sort    | `string`                                          | The sorting name of the artist.      | No       |
| artists        | [`ReleaseArtist[]`](#releaseartist)               | The preferred way to include artists | No       |
| year           | `u32`                                             | The year of the release.             | No       |
| date           | `ISODate`                                         | The date of the release.             | No       |
| country        | [`CountryCode`](#countrycode)                     | The country of the release.          | No       |
| script         | `string`                                          | The script of the release.           | No       |
| label_ids      | `string[]`                                        | The ULIDs of the labels.             | No       |
| genre_ids      | `string[]`                                        | The ULIDs of the genres.             | No       |
| tag_ids        | `string[]`                                        | The ULIDs of the tags.               | No       |
| mbz_id         | `string`                                          | The MusicBrainz ID of the release.   | No       |

### Notes

1. `artist` and `artists` are mutually exclusive.
   - If `artist` is present, `artist_id` will be required and `artists` will be ignored.
   - If `artists` is present, `artist` and `artist_id` will be ignored.

2. `artist_sort` field will be used for both `artist` and `artists` fields.

## Shared Types

### ReleaseType

Defines the primary types of a release.

- `album`
- `single`
- `ep`

### ReleaseTypeSecondary

Defines the secondary types of a release.
This is typically used for marking a special release.

- `compilation`
- `remix`
- `live`

### CountryCode

Defines the country of a release.

- `jp`
- `us`
- `xw`

### ScriptCode

Defines the script of a release.

- `latn`
- `jpan`

### ReleaseArtist

Convenience type for storing a release artist.

| Name | Type     | Description                                  | Required |
| ---- | -------- | -------------------------------------------- | -------- |
| id   | `string` | The ULID of the artist.                      | Yes      |
| name | `string` | The display name of the artist.              | Yes      |
| join | `string` | The join phrase to connect multiple artists. | No       |

## Pitfalls

1. A release can have multiple artists, but usually only one `ARTIST` tag is present in the metadata of a release. This makes splitting release artists difficult, as the joins between the artists are not always consistent. To handle cases like these, the `artist` and `artists` fields can be used.
   1. `artist`
      - Contains the display name of release artists, and pairs with the `artist_id` field.
      - Can be used when the release `artists` cannot be resolved correctly.
      - e.g. `milet×Aimer×幾田りら`, `Oh Shu & BIOMAN`, `tokiwa feat. 星宮とと` or `Teddyloid`.
   2. `artists`
      - Preferred way of storing release artists.
      - Contains the ULID, display name and a join phrase to connect mulitple artists.
      - e.g. `milet×Aimer×幾田りら` -> [Example](#artists-with-releaseartist)

## Examples

### `artists` with [`ReleaseArtist`](#releaseartist)

```js
[{ name: "millet", id: "", join: "×" }, { name: "Aimer", id: "", join: "×" }, { name: "幾田りら", id: "", join: "" }];
```

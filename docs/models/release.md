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
  - [Pitfalls](#pitfalls)
  - [Examples](#examples)
    - [`artists` with `InlinedArtist`](#artists-with-inlinedartist)

## Properties

| Name           | Type                                              | Description                          | Required |
| -------------- | ------------------------------------------------- | ------------------------------------ | -------- |
| title          | `string`                                          | The title of the release.            | true     |
| title_sort     | `string`                                          | The sorting title of the release.    | false    |
| type           | [`ReleaseType`](#releasetype)                     | The primary type of the release.     | true     |
| type_secondary | [`ReleaseTypeSecondary[]`](#releasetypesecondary) | The secondary types of the release.  | false    |
| artist_id      | `string`                                          | The ID of an artist.                 | false    |
| artist_sort    | `string`                                          | The sorting name of the artist.      | false    |
| artists        | [`InlinedArtist[]`](./generic.md#inlinedartist)   | The preferred way to include artists | false    |
| year           | `u32`                                             | The year of the release.             | false    |
| date           | `ISODate`                                         | The date of the release.             | false    |
| country        | [`CountryCode`](#countrycode)                     | The country of the release.          | false    |
| script         | `string`                                          | The script of the release.           | false    |
| label_ids      | `string[]`                                        | The IDs of the labels.               | false    |
| genre_ids      | `string[]`                                        | The IDs of the genres.               | false    |
| tag_ids        | `string[]`                                        | The IDs of the tags.                 | false    |
| total_tracks   | `u32`                                             | The total number of tracks.          | false    |
| total_discs    | `u32`                                             | The total number of discs.           | false    |
| mbz_id         | `string`                                          | The MusicBrainz release ID.          | false    |

### Notes

1. `artist_id` and `artists` are mutually exclusive.

2. `artist_sort` field will be used for both `artist_id` and `artists` fields.

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

## Pitfalls

1. A release can have multiple artists, but usually only one `ARTIST` tag is present in the metadata of a release. This makes splitting release artists difficult, as the joins between the artists are not always consistent. To handle cases like these, the `artist_id` and `artists` fields can be used.
   1. `artist_id`
      - Contains the single ID of the release artist.
      - Only used when the release `artists` cannot be resolved correctly.
      - e.g. `milet×Aimer×幾田りら`, `Oh Shu & BIOMAN`, `tokiwa feat. 星宮とと` or `Teddyloid`.
   2. `artists`
      - Preferred way of storing the release artists.
      - Contains the ID, display name and a join phrase to connect multiple artists.
      - e.g. `milet×Aimer×幾田りら` -> [Example](#artists-with-inlinedartist)

## Examples

### `artists` with [`InlinedArtist`](./generic.md#inlinedartist)

```js
[{ name: "millet", id: "", join: "×" }, { name: "Aimer", id: "", join: "×" }, { name: "幾田りら", id: "", join: "" }];
```

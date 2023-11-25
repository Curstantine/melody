# models/Release

Refers to a release done by an artist. A release can be an album, a single, an EP, a compilation, a live recording, etc.

- [models/Release](#modelsrelease)
  - [Properties](#properties)
  - [Shared Types](#shared-types)
    - [ReleaseType](#releasetype)
    - [ReleaseTypeSecondary](#releasetypesecondary)
    - [CountryCode](#countrycode)
    - [ScriptCode](#scriptcode)
  - [Pitfalls](#pitfalls)

## Properties

| Name           | Type                                              | Description                               | Required |
| -------------- | ------------------------------------------------- | ----------------------------------------- | -------- |
| title          | `string`                                          | The title of the release.                 | true     |
| title_sort     | `string`                                          | The sorting title of the release.         | false    |
| type           | [`ReleaseType`](#releasetype)                     | The primary type of the release.          | true     |
| type_secondary | [`ReleaseTypeSecondary[]`](#releasetypesecondary) | The secondary types of the release.       | false    |
| artists        | [`InlinedArtist[]`](./generic.md#inlinedartist)   | The references to the artists.            | true     |
| artist_sort    | `string`                                          | The sorting name of the artist.           | false    |
| year           | `i32`                                             | The year of the release.                  | false    |
| date           | `ISODate`                                         | The date of the release.                  | false    |
| country        | [`CountryCode`](#countrycode)                     | The country of the release.               | false    |
| script         | `string`                                          | The script of the release.                | false    |
| label_ids      | `u64[]`                                           | The IDs of the labels.                    | false    |
| genre_ids      | `u64[]`                                           | The IDs of the genres.                    | false    |
| tag_ids        | `u64[]`                                           | The IDs of the tags.                      | false    |
| cover_ids      | `string[]`                                        | The IDs of cover resources of this track. | false    |
| total_tracks   | `u32`                                             | The total number of tracks.               | false    |
| total_discs    | `u32`                                             | The total number of discs.                | false    |
| mbz_id         | `string`                                          | The MusicBrainz release ID.               | false    |

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

1. A release can have multiple artists, but usually only one `RELEASEARTIST` tag is present in the metadata of a track. This makes splitting release artists difficult, as the joins between the artists are not always consistent. In order to handle cases like these, the `artists` field doesn't guarantee that each entry refers to a single artist.

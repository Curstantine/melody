# models/Track

Refers to a music recording, a track.

## Properties

| Name          | Type                                            | Description                          | Required |
| ------------- | ----------------------------------------------- | ------------------------------------ | -------- |
| title         | string                                          | The title of the track               | true     |
| title_sort    | string                                          | The title of the track for sorting   | false    |
| track_number  | u16                                             | The track number                     | false    |
| disc_number   | u16                                             | The disc number                      | false    |
| original_date | ISODate                                         | The original release date            | false    |
| artist        | `string`                                        | The display name of the artists.     | false    |
| artist_id     | `string`                                        | The ULID of an artist.               | false    |
| artist_sort   | `string`                                        | The sorting name of the artist.      | false    |
| artists       | [`InlinedArtist[]`](./generic.md#inlinedartist) | The preferred way to include artists | false    |
| release_id    | `string`                                        | The ULID of the release.             | false    |
| composer_ids  | `string[]`                                      | The ULIDs of the composers.          | false    |
| producer_ids  | `string[]`                                      | The ULIDs of the producers.          | false    |
| genre_ids     | `string[]`                                      | The ULIDs of the genres.             | false    |
| tag_ids       | `string[]`                                      | The ULIDs of the tags.               | false    |
| mbz_id        | string                                          | The MusicBrainz recording ID         | false    |

### Notes

1. `artist` and `artists` are mutually exclusive.
   - If `artist` is present, `artist_id` will be required and `artists` will be ignored.
   - If `artists` is present, `artist` and `artist_id` will be ignored.

2. `artist_sort` field will be used for both `artist` and `artists` fields.

## Pitfalls

1. Check the first issue in [release/pitfalls](./release.md#pitfalls)

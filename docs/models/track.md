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
| artist_id     | `string`                                        | The ID of an artist.                 | false    |
| artist_sort   | `string`                                        | The sorting name of the artist.      | false    |
| artists       | [`InlinedArtist[]`](./generic.md#inlinedartist) | The preferred way to include artists | false    |
| release_id    | `string`                                        | The ID of the release.               | false    |
| composer_ids  | `string[]`                                      | The IDs of the composers.            | false    |
| producer_ids  | `string[]`                                      | The IDs of the producers.            | false    |
| genre_ids     | `string[]`                                      | The IDs of the genres.               | false    |
| tag_ids       | `string[]`                                      | The IDs of the tags.                 | false    |
| mbz_id        | string                                          | The MusicBrainz recording ID         | false    |

### Notes

1. `artist_id` and `artists` are mutually exclusive.

2. `artist_sort` field will be used for both `artist_id` and `artists` fields.

## Pitfalls

1. Check the first issue in [release/pitfalls](./release.md#pitfalls). The same applies here.

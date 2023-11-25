# models/Track

Refers to a music recording, a track.

## Properties

| Name          | Type                                            | Description                               | Required |
| ------------- | ----------------------------------------------- | ----------------------------------------- | -------- |
| title         | string                                          | The title of the track.                   | true     |
| title_sort    | string                                          | The title of the track for sorting.       | false    |
| track_number  | u16                                             | The track number.                         | false    |
| disc_number   | u16                                             | The disc number.                          | false    |
| original_date | `ISODate`                                       | The original release date.                | false    |
| artists       | [`InlinedArtist[]`](./generic.md#inlinedartist) | The references to the artists.            | false    |
| artist_sort   | `string`                                        | The sorting name of the artist.           | false    |
| release_id    | `string`                                        | The ID of the release.                    | false    |
| composer_ids  | `string[]`                                      | The IDs of the composers.                 | false    |
| producer_ids  | `string[]`                                      | The IDs of the producers.                 | false    |
| cover_ids     | `string[`                                       | The IDs of cover resources of this track. | false    |
| genre_ids     | `string[]`                                      | The IDs of the genres.                    | false    |
| tag_ids       | `string[]`                                      | The IDs of the tags.                      | false    |
| mbz_id        | `string`                                        | The MusicBrainz recording ID.             | false    |
| path          | `string`                                        | The path to the track.                    | true     |

## Pitfalls

1. A track can have multiple artists, but usually only one `ARTIST` tag is present in the metadata of a track. This makes splitting track artists difficult, as the joins between the artists are not always consistent. In order to handle cases like these, the `artists` field doesn't guarantee that each entry refers to a single artist.

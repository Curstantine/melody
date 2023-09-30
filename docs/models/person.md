# models/Person

Refers to a person. Be it an artist, composer or a producer.

## Properties

| Name      | Type                        | Description                       | Required |
| --------- | --------------------------- | --------------------------------- | -------- |
| id        | `string`                    | The ULID of the person.           | true     |
| name      | `string`                    | The name of the person.           | true     |
| name_sort | `string`                    | The sorting name of the person.   | false    |
| type      | [`PersonType`](#persontype) | The type of the person.           | true     |
| mbz_id    | `string`                    | The MusicBrainz ID of the person. | false    |

## Shared Types

### PersonType

Defines the type of a person.

- `artist`
- `composer`
- `producer`

## Pitfalls

1. Since the name property is not unique for every artist, it becomes difficult to distinguish between artists that share the same name. In cases like these, it's a good idea to depend on other identifiers like the person type or the MusicBrainz ID.

2. Most taggers doesn't properly support the `ARTIST` tag. For cases like these, MusicBrainz and other projects issue an `ARTISTS` tag with multiple values that hold the names of all the artists involved in a track. While this is a good replacement for _artists of a track_, there's no definitive way to resolve the artists engaged in a _release_. For this reason, a release cannot properly split the artists involved in it. See [release](./release.md#pitfalls) for more information.

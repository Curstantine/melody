# models/Resource

Refers to a digital resource related to an entity of any type. Typically, an artist profile image or a release cover art.

## Properties

| Name       | Type                            | Description                      | Required |
| ---------- | ------------------------------- | -------------------------------- | -------- |
| type       | [`ResourceType`](#resourcetype) | The type of resource this holds. | true     |
| path       | `string`                        | Path to this resource.           | true     |
| media_type | `MediaType`                     | MIME type of this resource.      | true     |
| hash       | `string`                        | A blake2 hash of this resource.  | true     |

## Shared Types

### MediaType

- jpeg
- png

### ResourceType

- `release`: A release cover art.
- `track`: A unique cover art for a track independent of the `release`.
- `artist`: An artist`s profile image.

# models/Resource

Refers to a digital resource related to an entity of any type. Typically, an artist profile image or a release cover art.

## Properties

| Name | Type                            | Description                      | Required |
| ---- | ------------------------------- | -------------------------------- | -------- |
| type | [`ResourceType`](#resourcetype) | The type of resource this holds. | true     |
| path | `string`                        | Path to this resource.           | true     |
| hash | `string`                        | A blake2 hash of this resource.  | true     |

## Shared Types

### ResourceType

- `release`: A release cover art.
  - `front_cover`: The front facing cover of a release. For a single release with multiple tracks pointing to many the first hit will be used as the front_cover and everything else will be used as track only covers.
  - `track`: A unique cover art for a track independent of the `front_cover`.
  - `other`: Exhaustive for the sake of displaying.
- `artist`: An artist`s profile image.

# models/Resource

Refers to a digital resource related to an entity of any type. Typically, an artist profile image or a release cover art.

## Properties

| Name | Type                            | Description                      | Required |
| ---- | ------------------------------- | -------------------------------- | -------- |
| type | [`ResourceType`](#resourcetype) | The type of resource this holds. | true     |
| path | `string`                        | Path to this resource.           | true     |

## Shared Types

### ResourceType

- `release`: A release cover art.
- `artist`: An artist`s profile image.

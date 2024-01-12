# models/Cover

A cover resource of sorts. Could be a release cover-art, track art, or an artist image.

## Properties

| Name       | Type                                | Description                                      | Required |
| ---------- | ----------------------------------- | ------------------------------------------------ | -------- |
| type       | [`CoverType`](#covertype)           | The type of cover this holds.                    | true     |
| media_type | [`CoverMediaType`](#covermediatype) | The media type of this cover.                    | true     |
| resolution | `(u16, u16)`                        | The (x, y) resolution of this cover.             | true     |
| comment    | `string`                            | A user comment embedded inside related resource. | false    |
| hash       | `string`                            | The BLAKE 3 hash of this resource.               | true     |

### Notes

1. Covers are stored in [`data_directory`](../spec.md#data-directory) under `/covers` directory, accompanied by its `type` sub-directory following the `${hash}.{ext}` format.
   - `type: track, hash: 123, media_type: png` -> `data_directory/covers/tracks/123.png`
   - `type: release, hash: 321, media_type: jpg` -> `data_directory/covers/release/321.png`
2. A cover should be accompanied by a thumb of the same image, if the resolution of the x axis higher than 512.
   - The thumb must be sized as x: 512 and y relative to x.
   - Must be stored in jpeg format, saved along with the original cover postfixed with `@512.{ext}`.

## Shared Types

### CoverType

- `release`: A release cover art.
- `track`: A unique cover art for a track independent of the `release`.
- `artist`: An artist`s profile image.

### CoverMediaType

- jpeg
- png

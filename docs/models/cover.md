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

1. Covers are stored in the [`data_directory`](../spec.md#data-directory) under `/covers` directory, following the `${hash}.{ext}` format:
   - `hash: 123, media_type: png` -> `data_directory/covers/123.png`
   - `hash: 321, media_type: jpg` -> `data_directory/covers/321.png`
2. Covers saved to disk are usually thumbs, but in cases where the source image is >= 512, the image is saved as is.
   - Must be sized as x: 512 and y relative to x.
   - PNG format must be used if the source is resized.
3. References in the model are for the source cover (pre-resize), so properties like `resolution` and `hash` are not of the thumb, but rather of the source cover that may or may not be locally saved.

## Shared Types

### CoverType

- `release`: A release cover art.
- `track`: A unique cover art for a track independent of the `release`.
- `artist`: An artist`s profile image.

### CoverMediaType

- jpeg
- png

# UVC30 FOV protocol notes

These notes document the small portion of the Yealink RC8 USB HID protocol used
by this project. They were derived from observed USB traffic and independent
analysis; no vendor source code is included.

## HID interface

- USB vendor ID: `0x6993`
- USB product ID: `0xb019`
- top-level usage: telephony headset (`0x000b:0x0005`)
- report ID: `0xc8`
- report size: 64 bytes

The UVC30 also exposes standalone vendor-defined HID collections. Camera
commands sent through those collections are ignored; RC8 camera control uses
the telephony top-level collection.

## Report layout

A camera request fits in one 64-byte report:

| Report bytes | Meaning |
| --- | --- |
| `0` | report ID `0xc8` |
| `1` | protocol 2, complete frame: `0x13` |
| `2` | logical message length: `48` |
| `3` | sequence: `0` |
| `4..20` | 16-byte RC8 message header |
| `20..52` | 32-byte camera payload |
| `52..64` | zero padding |

The message header contains a little-endian message ID at offset 0, transaction
ID at offset 2, payload length at offset 4, response status at offset 8, and
reserved zero fields elsewhere. Negative response statuses indicate failure.

The camera payload starts with a little-endian operation ID and signed value:

| Operation | Message | Value |
| --- | --- | --- |
| save parameters (`46`) | `0x0211` | `1` |
| get FOV (`56`) | `0x0212` | `-1` in the request; degrees in the reply |
| set FOV (`56`) | `0x0211` | `70`, `90`, or `120` |

Replies echo the message and transaction IDs. The tool reads the FOV back after
setting it instead of assuming that an acknowledged write was applied, then
sends the save-parameters operation.

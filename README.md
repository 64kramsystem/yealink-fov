# yealink-fov

`yealink-fov` is a small command-line tool for changing the field of view on a
Yealink UVC30 Desktop webcam without installing Yealink USB Connect.

It talks directly to the camera over USB HID and supports the three FOV values
offered by the camera: 70°, 90°, and 120°.

## Install

Install a Rust toolchain, then run:

```sh
cargo install --git https://github.com/64kramsystem/yealink-fov
```

Or build a local checkout:

```sh
cargo build --release
```

The code builds on Windows, Linux, and macOS. Protocol and hardware validation
were performed on macOS with a Yealink UVC30 Desktop (`6993:b019`). Other
Yealink models and product IDs are not currently supported.

## Use

Close Yealink USB Connect and applications actively capturing from the webcam,
such as OBS, then choose a field of view:

```sh
yealink-fov 70
yealink-fov 90
yealink-fov 120
```

The tool reads the setting back from the webcam, saves it on the device, and
exits with an error if the camera did not apply it.

If multiple UVC30 cameras are connected, select one by USB serial number:

```sh
yealink-fov --serial SERIAL 90
```

### Linux permissions

If opening the webcam fails without `sudo`, install the included udev rule:

```sh
sudo cp packaging/99-yealink-uvc30.rules /etc/udev/rules.d/
sudo udevadm control --reload-rules
sudo udevadm trigger
```

Then reconnect the camera.

The rule grants access to the active local desktop session through `uaccess`.
Headless or SSH-only Linux systems may require a site-specific group rule.

## Frame rate

This tool changes FOV only; it cannot add capture modes that the camera does not
advertise. Yealink's current UVC30 Desktop specifications list 4K, 1080p, and
720p output at 30 FPS, with a 30 FPS maximum. The tested camera likewise
advertised only 30 FPS formats to macOS, including at lower resolutions.

[Yealink UVC30 Desktop product page](https://www.yealink.com/en/product-detail/camera-uvc30-desktop)

## Scope

This project is an independent, minimal implementation of the USB messages
needed to set FOV. It contains no Yealink code or binaries and is not affiliated
with or endorsed by Yealink. The observed message format is documented in
[`docs/protocol.md`](docs/protocol.md).

## License

MIT

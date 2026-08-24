# yealink-fov

`yealink-fov` is a small command-line tool for changing camera settings on a
Yealink UVC30 Desktop webcam without installing Yealink USB Connect.

It talks directly to the camera over USB HID and supports field of view (70°,
90°, or 120°) and wide dynamic range (levels 0 through 5).

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

Close Yealink USB Connect, then choose a field of view:

```sh
yealink-fov 70
yealink-fov 90
yealink-fov 120
```

The explicit `fov` form is also accepted:

```sh
yealink-fov fov 90
```

Set wide dynamic range to any level from 0 through 5:

```sh
yealink-fov wdr 0
yealink-fov wdr 5
```

Video capture applications such as OBS can remain open. The tool reads each
setting back from the webcam, saves it on the device, and exits with an error
if the camera did not apply it.

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

## Scope

This project is an independent, minimal implementation of the USB messages
needed to set FOV and WDR. It contains no Yealink code or binaries and is not
affiliated with or endorsed by Yealink. The observed message format is
documented in [`docs/protocol.md`](docs/protocol.md).

## License

MIT

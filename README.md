# Simp

Simp is a fast and simple GPU-accelerated image manipulation program.

![Screenshot](images/screenshot.png)

## Goals

- Support of as many image formats as possible.
- 60 Hz. Avoid blocking for more than 16ms.
- Flicker free redrawing.
- Smooth resizing.
- Always use GPU-acceleration where possible.
- Desktop OS support.
- Basic image manipulation.

### Non Goals

- Powerful image manipulation (I am not making the next photoshop)
- Web/Mobile platform support.

## Supported Platforms

| OS      | Support                                                              |
| ------- | -------------------------------------------------------------------- |
| Linux   | The aur package is the only platform with all image formats enabled. |
| Windows | Windows does not have any optional formats enabled by default.       |
| MacOS   | MacOS compiles but is not well tested.                               |
| NetBSD  | Native package available.                                            |

## Supported Codecs

| Format    | Decoding                                                             | Encoding        |
| --------- | -------------------------------------------------------------------- | --------------- |
| PNG       | ✅                                                                    | ✅               |
| JPEG      | ✅ Baseline and progressive                                           | ✅ Baseline      |
| GIF       | ✅                                                                    | ✅               |
| BMP       | ✅                                                                    | ✅               |
| ICO       | ✅                                                                    | ✅               |
| TIFF      | ✅                                                                    | ✅               |
| WebP      | ✅ Converted to Rgba8                                                 | ✅ Lossless only |
| AVIF      | 🚧 Only 8-bit (1)                                                     | ❌               |
| PNM       | ✅                                                                    | ❌               |
| DDS       | ✅                                                                    | ❌               |
| TGA       | ✅                                                                    | ✅               |
| farbfeld  | ✅                                                                    | ✅               |
| SVG       | ✅ (2)                                                                | ❌               |
| PSD       | ✅                                                                    | ❌               |
| Raw       | ✅ Support from [rawloader](https://github.com/pedrocr/rawloader) (3) | ❌               |
| HEIF/HEIC | ✅ (4)                                                                | ❌               |
| JPEG XL   | ✅ (5)                                                                | ✅               |
| OpenEXR   | ✅                                                                    | ✅               |
| qoi       | ✅                                                                    | ✅               |
| hdr       | ✅                                                                    | ✅               |

1. Building with AVIF support requires the C library dav1d and is therefore not enabled by default.
2. SVGs are rastarized because Simp is primarily a bitmap image editor.
3. Most common cameras are supported but the colors may look weird because the standard curve may not fit all images.
4. HEIF/HEIC is only enabled on linux by default.
5. JPEG XL is only works well on linux currently.

## Keybinds

| Action           | Input                |
| ---------------- | :------------------- |
| Open image       | Ctrl + O             |
| Save as          | Ctrl + S             |
| Reload image     | F5                   |
| New window       | Ctrl + N             |
| Undo             | Ctrl + Z             |
| Redo             | Ctrl + Y             |
| Copy             | Ctrl + C             |
| Paste            | Ctrl + V             |
| Resize           | Ctrl + R             |
| Rotate left      | Q                    |
| Rotate right     | E                    |
| Zoom in          | - or Mousewheel up   |
| Zoom out         | + or Mousewheel down |
| Best fit         | Ctrl + B             |
| Largest fit      | Ctrl + L             |
| Crop             | Ctrl + X             |
| Fullscreen       | F11 or F             |
| Delete image     | Delete               |
| 100% - 900% Zoom | Ctrl + 1 - 9         |
| Previous image   | A or Left arrow      |
| Next image       | D or Right arrow     |

## Runtime dependencies
The dav1d library is required for AVIF support and libheif is required for heif/heic support.
```shell
pacman -S dav1d libheif
```

## Build dependencies

```shell
pacman -S git rust cargo-about nasm clang
```

## Installation

### Cargo

```shell
cargo install simp --locked
```

### Arch
```shell
paru -S simp
```

### NetBSD
A pre-compiled binary is available from the official repositories. To install it, simply run:
```
pkgin install simp
```

### Latest from github

```shell
cargo install --git https://github.com/Kl4rry/simp --locked
```

The latest build is very likely buggy and unfinished.
You can always also just grab the latest binary from actions build artifacts.

### Manual

Just download the exe from the releases tab. No actual installation is required.

### The name

It's an acronym (S)imple (i)mage (m)anipulation (p)rogram.

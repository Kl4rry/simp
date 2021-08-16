# Simp  

The (s)imple (i)mage (m)anipulation (p)rogram is a cross-platform image viewer focused on simplicity and speed.

![Screenshot](images/screenshot.png)

## Goals

* Support of as many image formats as possible.
* 60 Hz. Avoid blocking for more then 16ms.
* Flicker free redrawing.
* Desktop OS support.
* Basic image manipulation (Not reached).

### Non Goals

* Powerful image manipulation (I am not making the next photoshop)
* Web/Mobile platform support.

## Supported Platforms

| OS      | Support |
| ------- | :-----: |
| Windows |    ✅    |
| Linux   |    ✅    |
| MacOS   |    🆗    |

✅ = Tested and working  
🆗 = Untested but should work with minimal changes

## Supported Codecs

| Format   | Decoding                                                             | Encoding        |
| -------- | -------------------------------------------------------------------- | --------------- |
| PNG      | ✅                                                                    | ✅               |
| JPEG     | ✅ Baseline and progressive                                           | ✅ Baseline      |
| GIF      | ✅                                                                    | ✅               |
| BMP      | ✅                                                                    | ✅               |
| ICO      | ✅                                                                    | ✅               |
| TIFF     | ✅ Baseline(no fax support) + LZW + PackBits                          | ✅               |
| WebP     | ✅ Converted to Rgba8                                                 | ✅ Lossless only |
| AVIF     | ✅ Only 8-bit                                                         | ❌               |
| PNM      | ✅ PBM, PGM, PPM, standard PAM                                        | ❌               |
| DDS      | ✅ DXT1, DXT3, DXT5                                                   | ❌               |
| TGA      | ✅                                                                    | ✅               |
| farbfeld | ✅                                                                    | ✅               |
| SVG      | ✅ Rastarized at 96 dpi                                               | ❌               |
| PSD      | ✅                                                                    | ❌               |
| Raw      | ✅ Support from [rawloader](https://github.com/pedrocr/rawloader) (1) | ❌               |

1. Most common cameras are supported but the colors may look weird because the standard curve may not fit all images.

## Keybinds

| Action       | Input                |
| ------------ | :------------------- |
| Open image   | Ctrl + O             |
| Save as      | Ctrl + S             |
| Reload image | Ctrl + R             |
| New window   | Ctrl + N             |
| Undo         | Ctrl + Z             |
| Redo         | Ctrl + Y             |
| Copy         | Ctrl + C             |
| Paste        | Ctrl + V             |
| Rotate left  | Ctrl + Q             |
| Rotate right | Ctrl + E             |
| Zoom in      | - or Mousewheel up   |
| Zoom out     | + or Mousewheel down |
| Best fit     | E                    |
| Largest fit  | F                    |
| Crop         | Ctrl + X             |
| F11          | Fullscreen           |
| Delete image | Delete               |
| 1 - 9        | 100% - 900% Zoom     |

## System dependencies

System dependencies are only required at compile time.

### Linux

* libcairo2-dev
* libpango1.0-dev
* libgtk-3-dev
* libxcb-render0-dev
* libxcb-shape0-dev
* libxcb-xfixes0-dev

## Installation

### Cargo

```shell
cargo install simp
```

### Latest from github

```shell
cargo install --git https://github.com/Kl4rry/simp
```

The latest bulid is very likely buggy and unfinished.

### Manual

Just download the exe from the releases tab. No actual installation is required.

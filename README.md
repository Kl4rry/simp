# simp  
The (s)imple (im)age (p)rogram is a cross-platform image viewer focused on simplicity and speed.

![Screenshot](images/screenshot.png)

# Goals
* Support of as many image formats as possible
* 60 Hz (Nothing should ever block for more than 16ms)
* Flicker free redrawing
* Basic image manipulation (Not reached)

## Non Goals
* Powerful image manipulation (I am not making the next photoshop)

# Supported Platforms
| OS            | Support|
| ------------- |:------:|
| Windows       | ✅ |
| Linux         | ✅ |
| MacOS         | 🆗 |

✅ = Tested and working 🆗 = Untested but should work with minimal changes

# Supported Codecs
| Format | Support |
| ------ | -------- |
| PNG    | ✅ |
| JPEG   | Baseline and progressive |
| GIF    | ✅ |
| BMP    | ✅ |
| ICO    | ✅ |
| TIFF   | Baseline(no fax support) + LZW + PackBits |
| WebP   | ✅ |
| AVIF   | Only 8-bit |
| PNM    | PBM, PGM, PPM, standard PAM |
| DDS    | DXT1, DXT3, DXT5 |
| TGA    | ✅ |
| farbfeld | ✅ |
| SVG    | Rastarized at 96 dpi |
| PSD    | ✅ 
| Raw    | Support from [rawloader](https://github.com/pedrocr/rawloader) <sup>1</sup> |

<sup>1</sup> Most common cameras are supported but the colors may look weird because the standard curve may not fit all images.

# System dependencies
System dependencies are only required at compile time.
## Linux
* libcairo2-dev
* libpango1.0-dev
* libgtk-3-dev
* libxcb-render0-dev
* libxcb-shape0-dev
* libxcb-xfixes0-dev

# Installation
## Cargo
```shell
cargo install simp
```
## Latest from github
```shell
cargo install --git https://github.com/Kl4rry/simp
```
## Manual
Just download the exe from the releases tab. No actual installation is required.

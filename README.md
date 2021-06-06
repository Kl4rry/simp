# simp  
The (s)imple (im)age (p)rogram is a lightweight image viewer focused on simplicity and speed.

![Screenshot](images/screenshot.png)

# Goals
* Support of as many image formats as possible
* 60 Hz on all modern hardware
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
| PNG    | Yes |
| JPEG   | Baseline and progressive |
| GIF    | No animation |
| BMP    | Yes |
| ICO    | Yes |
| TIFF   | Baseline(no fax support) + LZW + PackBits |
| WebP   | Lossy(Luma channel only) |
| AVIF   | Only 8-bit |
| PNM    | PBM, PGM, PPM, standard PAM |
| DDS    | DXT1, DXT3, DXT5 |
| TGA    | Yes |
| farbfeld | Yes |
| svg    | Rastarized at 96 dpi |
| psd    | Yes |

# Installation
## Cargo
```shell
cargo install simp
```
## Manual
Just download the exe from the releases tab. No actual installation is required.

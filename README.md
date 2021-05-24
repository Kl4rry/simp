# simp  
The (s)imple (im)age (p)rogram is a lightweight image viewer.

## Supported Platforms
| OS            | Support|
| ------------- |:------:|
| Windows       | ✅ |
| Linux         | 🆗 |
| MacOS         | 🆗 |

✅ = Tested and working 🆗 = Untested but should work with minimal changes

## Supported Codecs
| Format | Decoding |
| ------ | -------- |
| PNG    | Yes |
| JPEG   | Baseline and progressive |
| GIF    | Yes |
| BMP    | Yes |
| ICO    | Yes |
| TIFF   | Baseline(no fax support) + LZW + PackBits |
| WebP   | Lossy(Luma channel only) |
| AVIF   | Only 8-bit |
| PNM    | PBM, PGM, PPM, standard PAM |
| DDS    | DXT1, DXT3, DXT5 |
| TGA    | Yes |
| farbfeld | Yes |

## Installation
### Cargo
```shell
cargo install simp
```

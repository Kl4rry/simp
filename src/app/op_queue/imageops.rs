use image::{
    DynamicImage, GenericImageView, ImageBuffer, Luma, LumaA, Pixel, Primitive, Rgb, Rgba,
};
use num_traits::{NumCast, ToPrimitive};

use crate::{max, min};

pub trait ToGrayScale {
    type SubPixel;
    fn to_gray_scale(&self) -> LumaA<Self::SubPixel>;
}

impl<T> ToGrayScale for Rgb<T>
where
    T: Primitive,
{
    type SubPixel = T;
    fn to_gray_scale(&self) -> LumaA<Self::SubPixel> {
        LumaA([to_gray_scale(&self.0), NumCast::from(1).unwrap()])
    }
}

impl<T> ToGrayScale for Rgba<T>
where
    T: Primitive,
{
    type SubPixel = T;
    fn to_gray_scale(&self) -> LumaA<Self::SubPixel> {
        LumaA([to_gray_scale(&self.0), self.0[3]])
    }
}

fn to_gray_scale<T: Primitive>(rgb: &[T]) -> T {
    const TO_LUMA: [f64; 3] = [0.299, 0.587, 0.114];
    let r: f64 = NumCast::from(rgb[0]).unwrap();
    let g: f64 = NumCast::from(rgb[1]).unwrap();
    let b: f64 = NumCast::from(rgb[2]).unwrap();

    let r = r * TO_LUMA[0];
    let g = g * TO_LUMA[1];
    let b = b * TO_LUMA[2];

    NumCast::from(r + g + b).unwrap()
}

impl<T> ToGrayScale for Luma<T>
where
    T: Primitive,
{
    type SubPixel = T;
    fn to_gray_scale(&self) -> LumaA<Self::SubPixel> {
        LumaA([self.0[0], NumCast::from(1).unwrap()])
    }
}

impl<T> ToGrayScale for LumaA<T>
where
    T: Primitive,
{
    type SubPixel = T;
    fn to_gray_scale(&self) -> LumaA<Self::SubPixel> {
        *self
    }
}

pub fn grayscale<Sub, OldPixel, I: GenericImageView<Pixel = OldPixel>>(
    image: &I,
) -> ImageBuffer<LumaA<Sub>, Vec<Sub>>
where
    Sub: Primitive,
    OldPixel: Pixel + ToGrayScale<SubPixel = Sub>,
{
    let (width, height) = image.dimensions();
    let mut out = ImageBuffer::new(width, height);

    for (x, y, pixel) in image.pixels() {
        let grayscale = pixel.to_gray_scale();
        out.put_pixel(x, y, grayscale);
    }

    out
}

pub struct Hsl {
    h: f64,
    s: f64,
    l: f64,
}

// from https://docs.rs/hsl/latest/src/hsl/lib.rs.html#1-206
pub fn rgb2hsl<Sub: Primitive + ToPrimitive>(rgb: Rgb<Sub>) -> Hsl {
    let mut h: f64;

    let max: f64 = NumCast::from(Sub::DEFAULT_MAX_VALUE).unwrap();
    let r: f64 = rgb.0[0].to_f64().unwrap() / max;
    let g: f64 = rgb.0[1].to_f64().unwrap() / max;
    let b: f64 = rgb.0[2].to_f64().unwrap() / max;

    let max = max!(max!(r, g), b);
    let min = min!(min!(r, g), b);

    // Luminosity is the average of the max and min rgb color intensities.
    let l = (max + min) / 2_f64;

    // Saturation
    let delta: f64 = max - min;
    if delta == 0_f64 {
        // it's gray
        return Hsl {
            h: 0_f64,
            s: 0_f64,
            l,
        };
    }

    // it's not gray
    let s = if l < 0.5_f64 {
        delta / (max + min)
    } else {
        delta / (2_f64 - max - min)
    };

    // Hue
    let r2 = (((max - r) / 6_f64) + (delta / 2_f64)) / delta;
    let g2 = (((max - g) / 6_f64) + (delta / 2_f64)) / delta;
    let b2 = (((max - b) / 6_f64) + (delta / 2_f64)) / delta;

    h = match max {
        x if x == r => b2 - g2,
        x if x == g => (1_f64 / 3_f64) + r2 - b2,
        _ => (2_f64 / 3_f64) + g2 - r2,
    };

    // Fix wraparounds
    if h < 0 as f64 {
        h += 1_f64;
    } else if h > 1_f64 {
        h -= 1_f64;
    }

    // Hue is precise to milli-degrees, e.g. `74.52deg`.
    let h_degrees = (h * 360_f64 * 100_f64).round() / 100_f64;
    Hsl { h: h_degrees, s, l }
}

// from https://docs.rs/hsl/latest/src/hsl/lib.rs.html#1-206
pub fn hsl2rgb<Sub: Primitive + ToPrimitive>(hsl: Hsl) -> Rgb<Sub> {
    let to_sub = |pre: f64| -> Sub {
        NumCast::from((pre * Sub::DEFAULT_MAX_VALUE.to_f64().unwrap()).round()).unwrap()
    };

    if hsl.s == 0.0 {
        // Achromatic, i.e., grey.
        let l: Sub = to_sub(hsl.l);
        return Rgb([l, l, l]);
    }

    let h = hsl.h / 360.0; // treat this as 0..1 instead of degrees
    let s = hsl.s;
    let l = hsl.l;

    let q = if l < 0.5 {
        l * (1.0 + s)
    } else {
        l + s - (l * s)
    };
    let p = 2.0 * l - q;

    Rgb([
        to_sub(hue_to_rgb(p, q, h + 1.0 / 3.0)),
        to_sub(hue_to_rgb(p, q, h)),
        to_sub(hue_to_rgb(p, q, h - 1.0 / 3.0)),
    ])
}

fn hue_to_rgb(p: f64, q: f64, t: f64) -> f64 {
    // Normalize
    let t = if t < 0.0 {
        t + 1.0
    } else if t > 1.0 {
        t - 1.0
    } else {
        t
    };

    if t < 1.0 / 6.0 {
        p + (q - p) * 6.0 * t
    } else if t < 1.0 / 2.0 {
        q
    } else if t < 2.0 / 3.0 {
        p + (q - p) * (2.0 / 3.0 - t) * 6.0
    } else {
        p
    }
}

fn luma2hsl<Sub: Primitive + ToPrimitive>(luma: Sub) -> Hsl {
    rgb2hsl(Rgb([luma, luma, luma]))
}

// TODO
// This function has a ton of duplication because the image crate is bad.
// When the Enlargable trait becomes public it can be generic over subpixel.
pub fn adjust_saturation_in_place(image: &mut DynamicImage, saturation: f64) {
    let (width, height) = image.dimensions();
    match image {
        DynamicImage::ImageRgb8(image) => {
            for y in 0..height {
                for x in 0..width {
                    let pixel = image.get_pixel(x, y);
                    let rgb = pixel.to_rgb();
                    let mut hsl = rgb2hsl(rgb);
                    let saturation = saturation / 100.0;
                    hsl.s = (hsl.s + saturation).clamp(0.0, 1.0);
                    let rgb: Rgb<u8> = hsl2rgb(hsl);
                    image.put_pixel(x, y, rgb);
                }
            }
        }
        DynamicImage::ImageRgba8(image) => {
            for y in 0..height {
                for x in 0..width {
                    let pixel = image.get_pixel(x, y);
                    let rgba = pixel.to_rgba();
                    let rgb = *Rgb::from_slice(&rgba.0[0..3]);
                    let mut hsl = rgb2hsl(rgb);
                    let saturation = saturation / 100.0;
                    hsl.s = (hsl.s + saturation).clamp(0.0, 1.0);
                    let rgb: Rgb<u8> = hsl2rgb(hsl);
                    image.put_pixel(x, y, Rgba([rgb[0], rgb[1], rgb[2], rgba[3]]));
                }
            }
        }
        DynamicImage::ImageRgb16(image) => {
            for y in 0..height {
                for x in 0..width {
                    let pixel = image.get_pixel(x, y);
                    let rgb = pixel.to_rgb();
                    let mut hsl = rgb2hsl(rgb);
                    let saturation = saturation / 100.0;
                    hsl.s = (hsl.s + saturation).clamp(0.0, 1.0);
                    let rgb: Rgb<u16> = hsl2rgb(hsl);
                    image.put_pixel(x, y, rgb);
                }
            }
        }
        DynamicImage::ImageRgba16(image) => {
            for y in 0..height {
                for x in 0..width {
                    let pixel = image.get_pixel(x, y);
                    let rgba = pixel.to_rgba();
                    let rgb = *Rgb::from_slice(&rgba.0[0..3]);
                    let mut hsl = rgb2hsl(rgb);
                    let saturation = saturation / 100.0;
                    hsl.s = (hsl.s + saturation).clamp(0.0, 1.0);
                    let rgb: Rgb<u16> = hsl2rgb(hsl);
                    image.put_pixel(x, y, Rgba([rgb[0], rgb[1], rgb[2], rgba[3]]));
                }
            }
        }
        DynamicImage::ImageRgb32F(image) => {
            for y in 0..height {
                for x in 0..width {
                    let pixel = image.get_pixel(x, y);
                    let rgb = pixel.to_rgb();
                    let mut hsl = rgb2hsl(rgb);
                    let saturation = saturation / 100.0;
                    hsl.s = (hsl.s + saturation).clamp(0.0, 1.0);
                    let rgb: Rgb<f32> = hsl2rgb(hsl);
                    image.put_pixel(x, y, rgb);
                }
            }
        }
        DynamicImage::ImageRgba32F(image) => {
            for y in 0..height {
                for x in 0..width {
                    let pixel = image.get_pixel(x, y);
                    let rgba = pixel.to_rgba();
                    let rgb = *Rgb::from_slice(&rgba.0[0..3]);
                    let mut hsl = rgb2hsl(rgb);
                    let saturation = saturation / 100.0;
                    hsl.s = (hsl.s + saturation).clamp(0.0, 1.0);
                    let rgb: Rgb<f32> = hsl2rgb(hsl);
                    image.put_pixel(x, y, Rgba([rgb[0], rgb[1], rgb[2], rgba[3]]));
                }
            }
        }
        _ => (),
    }
}

// TODO
// This function has the same problem was the one above.
pub fn lighten_in_place(image: &mut DynamicImage, value: f64) {
    let light = value / 100.0;
    let (width, height) = image.dimensions();
    match image {
        DynamicImage::ImageRgb8(image) => {
            for y in 0..height {
                for x in 0..width {
                    let rgba = *image.get_pixel(x, y);
                    let rgb = *Rgb::from_slice(&rgba.0[0..3]);
                    let mut hsl = rgb2hsl(rgb);
                    hsl.l = (hsl.l + light).clamp(0.0, 1.0);
                    let rgb: Rgb<u8> = hsl2rgb(hsl);
                    image.put_pixel(x, y, Rgb([rgb[0], rgb[1], rgb[2]]));
                }
            }
        }
        DynamicImage::ImageRgba8(image) => {
            for y in 0..height {
                for x in 0..width {
                    let rgba = *image.get_pixel(x, y);
                    let rgb = *Rgb::from_slice(&rgba.0[0..3]);
                    let mut hsl = rgb2hsl(rgb);
                    hsl.l = (hsl.l + light).clamp(0.0, 1.0);
                    let rgb: Rgb<u8> = hsl2rgb(hsl);
                    image.put_pixel(x, y, Rgba([rgb[0], rgb[1], rgb[2], rgba[3]]));
                }
            }
        }
        DynamicImage::ImageRgb16(image) => {
            for y in 0..height {
                for x in 0..width {
                    let rgba = *image.get_pixel(x, y);
                    let rgb = *Rgb::from_slice(&rgba.0[0..3]);
                    let mut hsl = rgb2hsl(rgb);
                    hsl.l = (hsl.l + light).clamp(0.0, 1.0);
                    let rgb: Rgb<u16> = hsl2rgb(hsl);
                    image.put_pixel(x, y, Rgb([rgb[0], rgb[1], rgb[2]]));
                }
            }
        }
        DynamicImage::ImageRgba16(image) => {
            for y in 0..height {
                for x in 0..width {
                    let rgba = *image.get_pixel(x, y);
                    let rgb = *Rgb::from_slice(&rgba.0[0..3]);
                    let mut hsl = rgb2hsl(rgb);
                    hsl.l = (hsl.l + light).clamp(0.0, 1.0);
                    let rgb: Rgb<u16> = hsl2rgb(hsl);
                    image.put_pixel(x, y, Rgba([rgb[0], rgb[1], rgb[2], rgba[3]]));
                }
            }
        }
        DynamicImage::ImageRgb32F(image) => {
            for y in 0..height {
                for x in 0..width {
                    let rgba = *image.get_pixel(x, y);
                    let rgb = *Rgb::from_slice(&rgba.0[0..3]);
                    let mut hsl = rgb2hsl(rgb);
                    hsl.l = (hsl.l + light).clamp(0.0, 1.0);
                    let rgb: Rgb<f32> = hsl2rgb(hsl);
                    image.put_pixel(x, y, Rgb([rgb[0], rgb[1], rgb[2]]));
                }
            }
        }
        DynamicImage::ImageRgba32F(image) => {
            for y in 0..height {
                for x in 0..width {
                    let rgba = *image.get_pixel(x, y);
                    let rgb = *Rgb::from_slice(&rgba.0[0..3]);
                    let mut hsl = rgb2hsl(rgb);
                    hsl.l = (hsl.l + light).clamp(0.0, 1.0);
                    let rgb: Rgb<f32> = hsl2rgb(hsl);
                    image.put_pixel(x, y, Rgba([rgb[0], rgb[1], rgb[2], rgba[3]]));
                }
            }
        }
        DynamicImage::ImageLuma8(image) => {
            for y in 0..height {
                for x in 0..width {
                    let luma = *image.get_pixel(x, y);
                    let mut hsl = luma2hsl(luma[0]);
                    hsl.l = (hsl.l + light).clamp(0.0, 1.0);
                    let luma: Luma<u8> = hsl2rgb(hsl).to_luma();
                    image.put_pixel(x, y, luma);
                }
            }
        }
        DynamicImage::ImageLumaA8(image) => {
            for y in 0..height {
                for x in 0..width {
                    let luma = *image.get_pixel(x, y);
                    let alpha = luma[1];
                    let mut hsl = luma2hsl(luma[0]);
                    hsl.l = (hsl.l + light).clamp(0.0, 1.0);
                    let luma: Luma<u8> = hsl2rgb(hsl).to_luma();
                    image.put_pixel(x, y, LumaA([luma[0], alpha]));
                }
            }
        }
        DynamicImage::ImageLuma16(image) => {
            for y in 0..height {
                for x in 0..width {
                    let luma = *image.get_pixel(x, y);
                    let mut hsl = luma2hsl(luma[0]);
                    hsl.l = (hsl.l + light).clamp(0.0, 1.0);
                    let luma: Luma<u16> = hsl2rgb(hsl).to_luma();
                    image.put_pixel(x, y, luma);
                }
            }
        }
        DynamicImage::ImageLumaA16(image) => {
            for y in 0..height {
                for x in 0..width {
                    let luma = *image.get_pixel(x, y);
                    let alpha = luma[1];
                    let mut hsl = luma2hsl(luma[0]);
                    hsl.l = (hsl.l + light).clamp(0.0, 1.0);
                    let luma: Luma<u16> = hsl2rgb(hsl).to_luma();
                    image.put_pixel(x, y, LumaA([luma[0], alpha]));
                }
            }
        }
        _ => (),
    }
}

use image::{GenericImageView, ImageBuffer, Luma, LumaA, Pixel, Primitive, Rgb, Rgba};
use num_traits::NumCast;

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

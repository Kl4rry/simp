use once_cell::sync::Lazy;

pub static JXL: Lazy<&[&'static str]> = Lazy::new(|| &["jxl"]);

pub static HEIF: Lazy<&[&'static str]> = Lazy::new(|| &["heif", "heic"]);

pub static RASTER: Lazy<&[&'static str]> = Lazy::new(|| {
    &[
        "png", "jpg", "jpeg", "jpe", "jif", "jfif", "gif", "bmp", "ico", "tiff", "webp", "avif",
        "pnm", "pbm", "pgm", "ppm", "pam", "dds", "tga", "ff", "farbfeld", "exr", "qoi",
    ]
});

pub static UNDETECTABLE_RASTER: Lazy<&[&'static str]> = Lazy::new(|| &["tga"]);

pub static VECTOR: Lazy<&[&'static str]> = Lazy::new(|| &["svg"]);

pub static RAW: Lazy<&[&'static str]> = Lazy::new(|| {
    &[
        "raw", "mrw", "arw", "srf", "sr2", "mef", "orf", "srw", "erf", "kdc", "dcs", "rw2", "raf",
        "dcr", "dng", "pef", "crw", "iiq", "3fr", "nrw", "nef", "mos", "cr2", "ari",
    ]
});

pub static PHOTOSHOP: Lazy<&[&'static str]> = Lazy::new(|| &["psd"]);

pub static EXTENSIONS: Lazy<Vec<&'static str>> = Lazy::new(|| {
    let mut vec: Vec<&'static str> = Vec::new();
    vec.extend(RASTER.iter());
    vec.extend(VECTOR.iter());
    vec.extend(PHOTOSHOP.iter());
    vec.extend(RAW.iter());
    #[cfg(feature = "heif")]
    vec.extend(HEIF.iter());
    #[cfg(feature = "jxl")]
    vec.extend(JXL.iter());
    vec
});

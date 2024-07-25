use std::sync::LazyLock;

pub static JXL: LazyLock<&[&'static str]> = LazyLock::new(|| &["jxl"]);

pub static HEIF: LazyLock<&[&'static str]> = LazyLock::new(|| &["heif", "heic"]);

pub static RASTER: LazyLock<&[&'static str]> = LazyLock::new(|| {
    &[
        "png", "jpg", "jpeg", "jpe", "jif", "jfif", "gif", "bmp", "ico", "tiff", "webp", "avif",
        "pnm", "pbm", "pgm", "ppm", "pam", "dds", "tga", "ff", "farbfeld", "exr", "qoi", "hdr",
    ]
});

pub static UNDETECTABLE_RASTER: LazyLock<&[&'static str]> = LazyLock::new(|| &["tga"]);

pub static VECTOR: LazyLock<&[&'static str]> = LazyLock::new(|| &["svg"]);

pub static RAW: LazyLock<&[&'static str]> = LazyLock::new(|| {
    &[
        "raw", "mrw", "arw", "srf", "sr2", "mef", "orf", "srw", "erf", "kdc", "dcs", "rw2", "raf",
        "dcr", "dng", "pef", "crw", "iiq", "3fr", "nrw", "nef", "mos", "cr2", "ari",
    ]
});

pub static PHOTOSHOP: LazyLock<&[&'static str]> = LazyLock::new(|| &["psd"]);

pub static EXTENSIONS: LazyLock<Vec<&'static str>> = LazyLock::new(|| {
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

use std::collections::HashSet;

use once_cell::sync::Lazy;

fn create_set(input: &[&'static str]) -> HashSet<&'static str> {
    input.iter().cloned().collect()
}

pub static RASTER: Lazy<HashSet<&'static str>> = Lazy::new(|| {
    create_set(&[
        "png", "jpg", "jpeg", "jpe", "jif", "jfif", "gif", "bmp", "ico", "tiff", "webp", "avif",
        "pnm", "pbm", "pgm", "ppm", "pam", "dds", "tga", "ff", "farbfeld", "exr",
    ])
});

pub static UNDETECTABLE_RASTER: Lazy<HashSet<&'static str>> = Lazy::new(|| create_set(&["tga"]));

pub static VECTOR: Lazy<HashSet<&'static str>> = Lazy::new(|| create_set(&["svg"]));

pub static RAW: Lazy<HashSet<&'static str>> = Lazy::new(|| {
    create_set(&[
        "raw", "mrw", "arw", "srf", "sr2", "mef", "orf", "srw", "erf", "kdc", "dcs", "rw2", "raf",
        "dcr", "dng", "pef", "crw", "iiq", "3fr", "nrw", "nef", "mos", "cr2", "ari",
    ])
});

pub static PHOTOSHOP: Lazy<HashSet<&'static str>> = Lazy::new(|| create_set(&["psd"]));

pub static EXTENSIONS: Lazy<HashSet<&'static str>> = Lazy::new(|| {
    let mut set: HashSet<&'static str> = HashSet::new();
    set.extend(RASTER.iter());
    set.extend(VECTOR.iter());
    set.extend(PHOTOSHOP.iter());
    set.extend(RAW.iter());
    set
});

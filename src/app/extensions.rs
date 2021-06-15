use lazy_static::*;
use std::collections::HashSet;

fn create_set(input: &[&'static str]) -> HashSet<&'static str> {
    input.iter().cloned().collect()
}

lazy_static! {
    pub static ref RASTER: HashSet<&'static str> = create_set(&[
        "png", "jpg", "jpeg", "jpe", "jif", "jfif", "gif", "bmp", "ico", "tiff", "webp", "avif",
        "pnm", "pbm", "pgm", "ppm", "pam", "dds", "tga", "ff"
    ]);
    pub static ref VECTOR: HashSet<&'static str> = create_set(&["svg"]);
    pub static ref RAW: HashSet<&'static str> = create_set(&[
        "raw", "mrw", "arw", "srf", "sr2", "mef", "orf", "srw", "erf", "kdc", "dcs", "rw2", "raf",
        "dcr", "dng", "pef", "crw", "iiq", "3fr", "nrw", "nef", "mos", "cr2", "ari"
    ]);
    pub static ref PHOTOSHOP: HashSet<&'static str> = create_set(&["psd"]);
    pub static ref EXTENSIONS: HashSet<&'static str> = {
        let mut set: HashSet<&'static str> = HashSet::new();
        set.extend(RASTER.iter());
        set.extend(VECTOR.iter());
        set.extend(PHOTOSHOP.iter());
        set.extend(RAW.iter());
        set
    };
}

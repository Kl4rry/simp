use image::imageops::FilterType;

pub const FILTERS: &[(FilterType, &str)] = &[
    (FilterType::Nearest, "Nearest Neighbor"),
    (FilterType::Triangle, "Linear Filter"),
    (FilterType::CatmullRom, "Cubic Filter"),
    (FilterType::Gaussian, "Gaussian Filter"),
    (FilterType::Lanczos3, "Lanczos"),
];

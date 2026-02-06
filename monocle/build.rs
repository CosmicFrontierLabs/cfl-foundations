fn main() {
    if std::env::var("CARGO_FEATURE_C_REFERENCE").is_ok() {
        cc::Build::new()
            .file("src/controllers/reference/CF_LOS_FB_40Hz.c")
            .include("src/controllers/reference")
            .compile("los_fb_reference");
    }
}

use rust_embed::RustEmbed;

/// Static assets embedded from ui/dist/
#[derive(RustEmbed)]
#[folder = "ui/dist/"]
pub struct Assets;

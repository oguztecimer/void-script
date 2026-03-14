use rust_embed::Embed;

#[derive(Embed)]
#[folder = "../../editor-ui/dist/"]
pub struct EditorAssets;

pub fn get_asset(path: &str) -> Option<(Vec<u8>, String)> {
    let path = if path.is_empty() || path == "/" {
        "index.html"
    } else {
        path.trim_start_matches('/')
    };

    EditorAssets::get(path).map(|file| {
        let mime = mime_guess::from_path(path)
            .first_or_octet_stream()
            .to_string();
        (file.data.to_vec(), mime)
    })
}

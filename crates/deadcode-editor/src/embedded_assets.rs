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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn woff2_mime_type_is_correct() {
        // Verify mime_guess resolves .woff2 to font/woff2.
        // This is critical: if mime_guess ever drops .woff2 support or
        // returns application/octet-stream, fonts will silently fail
        // in the wry WebView.
        let mime = mime_guess::from_path("font.woff2")
            .first_or_octet_stream()
            .to_string();
        assert_eq!(mime, "font/woff2", "mime_guess must resolve .woff2 to font/woff2");
    }

    #[test]
    fn common_web_mime_types_are_correct() {
        // Sanity check other critical asset types served by the custom protocol.
        // Notes on mime_guess behavior:
        // - .js returns "text/javascript" per RFC 9239 (current IANA type)
        // - .woff returns "application/font-woff" (older registered type)
        //   Both are accepted by modern browsers.
        let cases = vec![
            ("index.html", "text/html"),
            ("style.css", "text/css"),
            ("app.js", "text/javascript"),
            ("font.woff", "application/font-woff"),
        ];
        for (path, expected) in cases {
            let mime = mime_guess::from_path(path)
                .first_or_octet_stream()
                .to_string();
            assert_eq!(mime, expected, "MIME mismatch for {path}");
        }
    }
}

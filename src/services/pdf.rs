//! Renders PDF response bodies to bitmaps for preview, using a bundled
//! (auto-downloaded, cached) native pdfium library.

use std::sync::OnceLock;

use pdfium_render::prelude::*;

static PDFIUM: OnceLock<Result<Pdfium, String>> = OnceLock::new();

fn pdfium() -> Result<&'static Pdfium, String> {
    PDFIUM
        .get_or_init(|| pdfium_bundled::bind_pdfium_silent().map_err(|e| e.to_string()))
        .as_ref()
        .map_err(String::clone)
}

/// A single rendered PDF page as a raw RGBA pixel buffer.
pub struct RenderedPage {
    pub width: u32,
    pub height: u32,
    pub rgba: Vec<u8>,
}

/// Returns the number of pages in the PDF.
pub fn page_count(bytes: &[u8]) -> Result<usize, String> {
    let pdfium = pdfium()?;
    let document = pdfium.load_pdf_from_byte_slice(bytes, None).map_err(|e| e.to_string())?;
    Ok(document.pages().len() as usize)
}

/// Renders one page, scaled to `target_width` pixels wide (height follows the
/// page's own aspect ratio).
pub fn render_page(
    bytes: &[u8],
    page_index: usize,
    target_width: i32,
) -> Result<RenderedPage, String> {
    let pdfium = pdfium()?;
    let document = pdfium.load_pdf_from_byte_slice(bytes, None).map_err(|e| e.to_string())?;
    let page = document
        .pages()
        .get(page_index as i32)
        .map_err(|e| e.to_string())?;

    let target_height =
        ((target_width as f32) * page.height().value / page.width().value.max(1.0)) as i32;

    let bitmap = page
        .render(target_width.max(1), target_height.max(1), None)
        .map_err(|e| e.to_string())?;
    let image = bitmap.as_image().map_err(|e| e.to_string())?.into_rgba8();
    let (width, height) = image.dimensions();

    Ok(RenderedPage { width, height, rgba: image.into_raw() })
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Live-network test: downloads/caches the real pdfium binary on first
    /// run. `#[ignore]`d so normal `cargo test` doesn't need network access.
    /// Run with: cargo test --bins pdf:: -- --ignored --nocapture
    #[test]
    #[ignore]
    fn renders_a_generated_minimal_pdf() {
        let pdf_bytes = build_minimal_pdf();

        let count = page_count(&pdf_bytes).expect("page count");
        assert_eq!(count, 1);

        let page = render_page(&pdf_bytes, 0, 300).expect("render");
        assert_eq!(page.width, 300);
        assert!(page.height > 0);
        assert_eq!(page.rgba.len(), (page.width * page.height * 4) as usize);
    }

    /// Builds the same minimal spec-valid PDF as `scripts/test_file_server.py`
    /// (byte-offsets computed programmatically, so it's guaranteed valid).
    fn build_minimal_pdf() -> Vec<u8> {
        let stream_content = b"BT /F1 18 Tf 20 50 Td (hello) Tj ET".to_vec();
        let objects: Vec<Vec<u8>> = vec![
            b"<< /Type /Catalog /Pages 2 0 R >>".to_vec(),
            b"<< /Type /Pages /Kids [3 0 R] /Count 1 >>".to_vec(),
            b"<< /Type /Page /Parent 2 0 R /MediaBox [0 0 220 100] /Resources << /Font << /F1 4 0 R >> >> /Contents 5 0 R >>".to_vec(),
            b"<< /Type /Font /Subtype /Type1 /BaseFont /Helvetica >>".to_vec(),
            [
                format!("<< /Length {} >>\nstream\n", stream_content.len()).into_bytes(),
                stream_content,
                b"\nendstream".to_vec(),
            ]
            .concat(),
        ];

        let mut parts: Vec<u8> = b"%PDF-1.4\n".to_vec();
        let mut offsets = vec![0usize];
        for (i, obj) in objects.iter().enumerate() {
            offsets.push(parts.len());
            parts.extend_from_slice(format!("{} 0 obj\n", i + 1).as_bytes());
            parts.extend_from_slice(obj);
            parts.extend_from_slice(b"\nendobj\n");
        }
        let xref_offset = parts.len();
        let count = objects.len() + 1;
        parts.extend_from_slice(format!("xref\n0 {count}\n0000000000 65535 f \n").as_bytes());
        for off in &offsets[1..] {
            parts.extend_from_slice(format!("{off:010} 00000 n \n").as_bytes());
        }
        parts.extend_from_slice(
            format!("trailer\n<< /Size {count} /Root 1 0 R >>\nstartxref\n{xref_offset}\n%%EOF")
                .as_bytes(),
        );
        parts
    }
}

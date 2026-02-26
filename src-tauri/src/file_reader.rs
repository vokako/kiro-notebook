use std::fs;
use std::path::Path;

pub fn read_file(path: &str) -> Result<String, String> {
    let path = Path::new(path);
    let ext = path
        .extension()
        .and_then(|e| e.to_str())
        .unwrap_or("")
        .to_lowercase();

    match ext.as_str() {
        "pdf" => read_pdf(path),
        "docx" => read_docx(path),
        "md" | "txt" | "html" => {
            fs::read_to_string(path).map_err(|e| format!("Failed to read file: {}", e))
        }
        _ => Err(format!("Unsupported file type: {}", ext)),
    }
}

fn read_pdf(path: &Path) -> Result<String, String> {
    // 使用 macOS 原生 PDFKit 提取文本，兼容性最好
    #[cfg(target_os = "macos")]
    {
        let output = std::process::Command::new("swift")
            .arg("-e")
            .arg(format!(
                r#"import PDFKit
if let doc = PDFDocument(url: URL(fileURLWithPath: {:?})) {{
    var text = ""
    for i in 0..<doc.pageCount {{
        if let page = doc.page(at: i), let s = page.string {{
            text += s + "\n"
        }}
    }}
    print(text)
}}"#,
                path.to_str().unwrap_or("")
            ))
            .output()
            .map_err(|e| format!("Failed to run PDF extractor: {}", e))?;

        if output.status.success() {
            let text = String::from_utf8_lossy(&output.stdout).to_string();
            if !text.trim().is_empty() {
                return Ok(text);
            }
        }
    }

    // 非 macOS 或 PDFKit 失败时，回退到 lopdf 逐页提取
    let bytes = fs::read(path).map_err(|e| format!("Failed to read PDF: {}", e))?;
    let doc = lopdf::Document::load_mem(&bytes)
        .map_err(|e| format!("Failed to parse PDF: {}", e))?;
    let pages = doc.get_pages();
    let mut page_numbers: Vec<u32> = pages.keys().cloned().collect();
    page_numbers.sort();

    let mut text = String::new();
    for &pn in &page_numbers {
        if let Ok(page_text) = doc.extract_text(&[pn]) {
            text.push_str(&page_text);
        }
    }

    if text.is_empty() {
        Err("Failed to extract any text from PDF".to_string())
    } else {
        Ok(text)
    }
}

fn read_docx(path: &Path) -> Result<String, String> {
    let bytes = fs::read(path).map_err(|e| format!("Failed to read DOCX: {}", e))?;
    let doc = docx_rs::read_docx(&bytes).map_err(|e| format!("Failed to parse DOCX: {}", e))?;

    let mut text = String::new();
    for child in doc.document.children {
        if let docx_rs::DocumentChild::Paragraph(p) = child {
            for child in &p.children {
                if let docx_rs::ParagraphChild::Run(run) = child {
                    for child in &run.children {
                        if let docx_rs::RunChild::Text(t) = child {
                            text.push_str(&t.text);
                        }
                    }
                }
            }
            text.push('\n');
        }
    }
    Ok(text)
}

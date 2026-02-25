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
    let bytes = fs::read(path).map_err(|e| format!("Failed to read PDF: {}", e))?;
    pdf_extract::extract_text_from_mem(&bytes)
        .map_err(|e| format!("Failed to extract PDF text: {}", e))
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

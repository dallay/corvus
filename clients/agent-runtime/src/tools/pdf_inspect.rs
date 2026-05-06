use super::traits::{Tool, ToolResult};
use crate::security::SecurityPolicy;
use async_trait::async_trait;
use serde_json::json;
use std::sync::Arc;

const MAX_PDF_SIZE_BYTES: u64 = 50 * 1024 * 1024;

/// Inspect, classify, and extract text from a PDF file.
///
/// Detects whether the PDF is text-based, scanned, image-based, or mixed, and
/// converts extractable text to Markdown. Returns a structured result with the
/// detected type, page count, pages needing OCR, and optional Markdown output.
pub struct PdfInspectTool {
    security: Arc<SecurityPolicy>,
}

impl PdfInspectTool {
    pub fn new(security: Arc<SecurityPolicy>) -> Self {
        Self { security }
    }
}

#[async_trait]
impl Tool for PdfInspectTool {
    fn name(&self) -> &str {
        "pdf_inspect"
    }

    fn description(&self) -> &str {
        "Inspect and extract text from a PDF file. Detects whether the PDF is \
        text-based, scanned, image-based, or mixed, and converts extractable \
        text to Markdown. Returns PDF type, page count, pages needing OCR, \
        title, and Markdown content when available."
    }

    fn parameters_schema(&self) -> serde_json::Value {
        json!({
            "type": "object",
            "properties": {
                "path": {
                    "type": "string",
                    "description": "Relative path to the PDF file within the workspace"
                },
                "extract_text": {
                    "type": "boolean",
                    "description": "Whether to extract and convert text to Markdown (default: true). \
                                   Set to false for a fast metadata-only classification."
                }
            },
            "required": ["path"]
        })
    }

    async fn execute(&self, args: serde_json::Value) -> anyhow::Result<ToolResult> {
        let path = args
            .get("path")
            .and_then(|v| v.as_str())
            .ok_or_else(|| anyhow::anyhow!("Missing 'path' parameter"))?;

        let extract_text = args
            .get("extract_text")
            .and_then(|v| v.as_bool())
            .unwrap_or(true);

        if self.security.is_rate_limited() {
            return Ok(ToolResult {
                success: false,
                output: String::new(),
                error: Some("Rate limit exceeded: too many actions in the last hour".into()),
                structured: None,
            });
        }

        if !self.security.is_path_allowed(path) {
            return Ok(ToolResult {
                success: false,
                output: String::new(),
                error: Some(format!("Path not allowed by security policy: {path}")),
                structured: None,
            });
        }

        if !self.security.record_action() {
            return Ok(ToolResult {
                success: false,
                output: String::new(),
                error: Some("Rate limit exceeded: action budget exhausted".into()),
                structured: None,
            });
        }

        let full_path = self.security.workspace_dir.join(path);

        let resolved_path = match tokio::fs::canonicalize(&full_path).await {
            Ok(p) => p,
            Err(e) => {
                return Ok(ToolResult {
                    success: false,
                    output: String::new(),
                    error: Some(format!("Failed to resolve file path: {e}")),
                    structured: None,
                });
            }
        };

        if !self.security.is_resolved_path_allowed(&resolved_path) {
            return Ok(ToolResult {
                success: false,
                output: String::new(),
                error: Some(format!(
                    "Resolved path escapes workspace: {}",
                    resolved_path.display()
                )),
                structured: None,
            });
        }

        match tokio::fs::metadata(&resolved_path).await {
            Ok(meta) => {
                if meta.len() > MAX_PDF_SIZE_BYTES {
                    return Ok(ToolResult {
                        success: false,
                        output: String::new(),
                        error: Some(format!(
                            "File too large: {} bytes (limit: {MAX_PDF_SIZE_BYTES} bytes)",
                            meta.len()
                        )),
                        structured: None,
                    });
                }
            }
            Err(e) => {
                return Ok(ToolResult {
                    success: false,
                    output: String::new(),
                    error: Some(format!("Failed to read file metadata: {e}")),
                    structured: None,
                });
            }
        }

        let path_clone = resolved_path.clone();
        let result = tokio::task::spawn_blocking(move || {
            if extract_text {
                pdf_inspector::process_pdf(&path_clone)
            } else {
                pdf_inspector::process_pdf_with_options(
                    &path_clone,
                    pdf_inspector::PdfOptions::detect_only(),
                )
            }
        })
        .await;

        match result {
            Ok(Ok(info)) => {
                let pdf_type = format!("{:?}", info.pdf_type);
                let pages_needing_ocr = info.pages_needing_ocr.clone();
                let markdown = info.markdown.clone().unwrap_or_default();

                let output = if extract_text && !markdown.is_empty() {
                    markdown.clone()
                } else {
                    format!(
                        "PDF type: {pdf_type}\nPages: {}\nProcessing time: {}ms",
                        info.page_count, info.processing_time_ms
                    )
                };

                Ok(ToolResult {
                    success: true,
                    output,
                    error: None,
                    structured: Some(json!({
                        "pdf_type": pdf_type,
                        "page_count": info.page_count,
                        "processing_time_ms": info.processing_time_ms,
                        "pages_needing_ocr": pages_needing_ocr,
                        "title": info.title,
                        "has_markdown": info.markdown.is_some(),
                    })),
                })
            }
            Ok(Err(e)) => Ok(ToolResult {
                success: false,
                output: String::new(),
                error: Some(format!("Failed to process PDF: {e}")),
                structured: None,
            }),
            Err(e) => Ok(ToolResult {
                success: false,
                output: String::new(),
                error: Some(format!("PDF processing task panicked: {e}")),
                structured: None,
            }),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::security::{AutonomyLevel, SecurityPolicy};
    use serde_json::json;

    fn test_security(workspace: std::path::PathBuf) -> Arc<SecurityPolicy> {
        Arc::new(SecurityPolicy {
            autonomy: AutonomyLevel::Supervised,
            workspace_dir: workspace,
            ..SecurityPolicy::default()
        })
    }

    fn test_security_with(
        workspace: std::path::PathBuf,
        max_actions_per_hour: u32,
    ) -> Arc<SecurityPolicy> {
        Arc::new(SecurityPolicy {
            autonomy: AutonomyLevel::Supervised,
            workspace_dir: workspace,
            max_actions_per_hour,
            ..SecurityPolicy::default()
        })
    }

    #[test]
    fn pdf_inspect_name() {
        let tool = PdfInspectTool::new(test_security(std::env::temp_dir()));
        assert_eq!(tool.name(), "pdf_inspect");
    }

    #[test]
    fn pdf_inspect_schema_has_path() {
        let tool = PdfInspectTool::new(test_security(std::env::temp_dir()));
        let schema = tool.parameters_schema();
        assert!(schema["properties"]["path"].is_object());
        assert!(schema["required"]
            .as_array()
            .unwrap()
            .contains(&json!("path")));
    }

    #[test]
    fn pdf_inspect_schema_has_extract_text() {
        let tool = PdfInspectTool::new(test_security(std::env::temp_dir()));
        let schema = tool.parameters_schema();
        assert!(schema["properties"]["extract_text"].is_object());
        assert_eq!(schema["properties"]["extract_text"]["type"], "boolean");
    }

    #[tokio::test]
    async fn pdf_inspect_missing_path_param() {
        let tool = PdfInspectTool::new(test_security(std::env::temp_dir()));
        let result = tool.execute(json!({})).await;
        assert!(result.is_err());
    }

    #[tokio::test]
    async fn pdf_inspect_blocks_path_traversal() {
        let dir = std::env::temp_dir().join("corvus_test_pdf_inspect_traversal");
        let _ = tokio::fs::remove_dir_all(&dir).await;
        tokio::fs::create_dir_all(&dir).await.unwrap();

        let tool = PdfInspectTool::new(test_security(dir.clone()));
        let result = tool
            .execute(json!({"path": "../../../etc/passwd"}))
            .await
            .unwrap();
        assert!(!result.success);
        assert!(result.error.as_ref().unwrap().contains("not allowed"));

        let _ = tokio::fs::remove_dir_all(&dir).await;
    }

    #[tokio::test]
    async fn pdf_inspect_blocks_absolute_path() {
        let tool = PdfInspectTool::new(test_security(std::env::temp_dir()));
        let result = tool
            .execute(json!({"path": "/etc/passwd"}))
            .await
            .unwrap();
        assert!(!result.success);
        assert!(result.error.as_ref().unwrap().contains("not allowed"));
    }

    #[tokio::test]
    async fn pdf_inspect_nonexistent_file() {
        let dir = std::env::temp_dir().join("corvus_test_pdf_inspect_missing");
        let _ = tokio::fs::remove_dir_all(&dir).await;
        tokio::fs::create_dir_all(&dir).await.unwrap();

        let tool = PdfInspectTool::new(test_security(dir.clone()));
        let result = tool
            .execute(json!({"path": "nope.pdf"}))
            .await
            .unwrap();
        assert!(!result.success);
        assert!(result.error.as_ref().unwrap().contains("Failed to resolve"));

        let _ = tokio::fs::remove_dir_all(&dir).await;
    }

    #[tokio::test]
    async fn pdf_inspect_blocks_when_rate_limited() {
        let dir = std::env::temp_dir().join("corvus_test_pdf_inspect_rate_limited");
        let _ = tokio::fs::remove_dir_all(&dir).await;
        tokio::fs::create_dir_all(&dir).await.unwrap();

        let tool = PdfInspectTool::new(test_security_with(dir.clone(), 0));
        let result = tool
            .execute(json!({"path": "test.pdf"}))
            .await
            .unwrap();
        assert!(!result.success);
        assert!(result
            .error
            .as_deref()
            .unwrap_or("")
            .contains("Rate limit exceeded"));

        let _ = tokio::fs::remove_dir_all(&dir).await;
    }

    #[tokio::test]
    async fn pdf_inspect_rejects_oversized_file() {
        let dir = std::env::temp_dir().join("corvus_test_pdf_inspect_large");
        let _ = tokio::fs::remove_dir_all(&dir).await;
        tokio::fs::create_dir_all(&dir).await.unwrap();

        // Create a fake file just over 50 MB
        let big = vec![b'x'; 50 * 1024 * 1024 + 1];
        tokio::fs::write(dir.join("huge.pdf"), &big).await.unwrap();

        let tool = PdfInspectTool::new(test_security(dir.clone()));
        let result = tool
            .execute(json!({"path": "huge.pdf"}))
            .await
            .unwrap();
        assert!(!result.success);
        assert!(result.error.as_ref().unwrap().contains("File too large"));

        let _ = tokio::fs::remove_dir_all(&dir).await;
    }
}

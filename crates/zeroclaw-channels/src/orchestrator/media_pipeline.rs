//! Automatic media understanding pipeline for inbound channel messages.
//!
//! Pre-processes media attachments (audio, images, video) before the agent sees
//! the message, enriching the text with human-readable annotations:
//!
//! - **Audio**: transcribed via the existing [`super::transcription`] infrastructure,
//!   prepended as `[Audio transcription: ...]`.
//! - **Images**: when a vision-capable provider is active, described as `[Image: <description>]`.
//!   Falls back to `[Image: attached]` when vision is unavailable.
//! - **Video**: summarised as `[Video summary: ...]` when an API is available,
//!   otherwise `[Video: attached]`.
//!
//! The pipeline is **opt-in** via `[media_pipeline] enabled = true` in config.

use base64::{Engine as _, engine::general_purpose::STANDARD};
use std::borrow::Cow;
use std::sync::Arc;
use zeroclaw_api::provider::{ChatMessage, ChatRequest, Provider};
use zeroclaw_config::schema::{MediaPipelineConfig, TranscriptionConfig};

// Re-export media types from zeroclaw-types for backwards compatibility.
pub use zeroclaw_api::media::{MediaAttachment, MediaKind};

/// The media understanding pipeline.
///
/// Consumes a message's text and attachments, returning enriched text with
/// media annotations prepended.
pub struct MediaPipeline<'a> {
    config: &'a MediaPipelineConfig,
    transcription_config: &'a TranscriptionConfig,
    vision_available: bool,
    vision_provider: Option<Arc<dyn Provider>>,
    vision_model: Option<String>,
}

impl<'a> MediaPipeline<'a> {
    /// Create a new pipeline. `vision_available` indicates whether the current
    /// provider supports vision (image description).
    pub fn new(
        config: &'a MediaPipelineConfig,
        transcription_config: &'a TranscriptionConfig,
        vision_available: bool,
        vision_provider: Option<Arc<dyn Provider>>,
        vision_model: Option<String>,
    ) -> Self {
        Self {
            config,
            transcription_config,
            vision_available,
            vision_provider,
            vision_model,
        }
    }

    /// Process a message's attachments and return enriched text.
    ///
    /// If the pipeline is disabled via config, returns `original_text` unchanged.
    pub async fn process(
        &self,
        original_text: &str,
        attachments: &mut Vec<MediaAttachment>,
    ) -> String {
        if !self.config.enabled || attachments.is_empty() {
            return original_text.to_string();
        }

        let mut annotations = Vec::new();
        let mut to_remove = Vec::new();

        for (idx, attachment) in attachments.iter().enumerate() {
            match attachment.kind() {
                MediaKind::Audio if self.config.transcribe_audio => {
                    let annotation = self.process_audio(attachment).await;
                    annotations.push(annotation);
                }
                MediaKind::Image if self.config.describe_images => {
                    let (annotation, transcribed) = self.process_image(attachment).await;
                    annotations.push(annotation);
                    if transcribed {
                        to_remove.push(idx);
                    }
                }
                MediaKind::Video if self.config.summarize_video => {
                    let annotation = self.process_video(attachment);
                    annotations.push(annotation);
                }
                MediaKind::Unknown => {
                    let annotation = self.process_file(attachment);
                    annotations.push(annotation);
                }
                _ => {}
            }
        }

        // Remove transcribed attachments from the end to preserve indices
        for idx in to_remove.into_iter().rev() {
            attachments.remove(idx);
        }

        if annotations.is_empty() {
            return original_text.to_string();
        }

        let mut enriched = String::with_capacity(
            annotations.iter().map(|a| a.len() + 1).sum::<usize>() + original_text.len() + 2,
        );

        for annotation in &annotations {
            enriched.push_str(annotation);
            enriched.push('\n');
        }

        if !original_text.is_empty() {
            enriched.push('\n');
            enriched.push_str(original_text);
        }

        enriched.trim().to_string()
    }

    /// Transcribe an audio attachment using the existing transcription infra.
    async fn process_audio(&self, attachment: &MediaAttachment) -> String {
        if !self.transcription_config.enabled {
            return "[Audio: attached]".to_string();
        }

        match super::transcription::transcribe_audio(
            attachment.data.clone(),
            &attachment.file_name,
            self.transcription_config,
        )
        .await
        {
            Ok(text) => {
                let trimmed = text.trim();
                if trimmed.is_empty() {
                    "[Audio transcription: (empty)]".to_string()
                } else {
                    format!("[Audio transcription: {trimmed}]")
                }
            }
            Err(err) => {
                tracing::warn!(
                    file = %attachment.file_name,
                    error = %err,
                    "Media pipeline: audio transcription failed"
                );
                "[Audio: transcription failed]".to_string()
            }
        }
    }

    /// Describe an image attachment.
    ///
    /// When vision is available natively on the main provider, the image will be passed
    /// through as an `[IMAGE:]` marker and described by the model in the normal flow.
    /// If native vision is unavailable but a dedicated vision provider is configured,
    /// we transcribe/describe the image asynchronously and replace it with its text description.
    async fn process_image(&self, attachment: &MediaAttachment) -> (String, bool) {
        let (mime, data) = image_payload_for_vision(attachment);
        let b64 = STANDARD.encode(data.as_ref());

        // Native vision check: if active provider supports vision natively and no override is set
        if self.vision_available && self.vision_provider.is_none() {
            return (
                format!(
                    "[Image: {} attached, will be processed by vision model]\n[IMAGE:data:{};base64,{}]",
                    attachment.file_name, mime, b64
                ),
                false, // Keep attachment for native processing
            );
        }

        // Asynchronous image description check: if vision provider override is configured
        if let Some(ref provider) = self.vision_provider
            && let Some(ref model) = self.vision_model
        {
            let prompt = format!(
                "[IMAGE:data:{};base64,{}]\nDescribe this image in detail. Focus on any visible text, code, or terminal errors. Be concise (max 500 characters).",
                mime, b64
            );
            let messages = vec![ChatMessage::user(prompt)];
            let req = ChatRequest {
                messages: &messages,
                tools: None,
            };

            tracing::info!(
                file = %attachment.file_name,
                model = %model,
                "Media pipeline: transcribing image via vision provider"
            );

            match provider.chat(req, model, None).await {
                Ok(response) => {
                    let description = response.text_or_empty().trim();
                    if !description.is_empty() {
                        return (
                            format!(
                                "[Image description ({}): {}]",
                                attachment.file_name, description
                            ),
                            true, // Transcribed: remove attachment
                        );
                    }
                }
                Err(err) => {
                    tracing::warn!(
                        file = %attachment.file_name,
                        error = %err,
                        "Media pipeline: image transcription failed"
                    );
                }
            }
        }

        // Fallback: keep basic placeholder, keep attachment
        (format!("[Image: {} attached]", attachment.file_name), false)
    }

    /// Summarize a video attachment.
    ///
    /// Video analysis requires external APIs not currently integrated.
    /// For now we add a placeholder annotation.
    fn process_video(&self, attachment: &MediaAttachment) -> String {
        format!("[Video: {} attached]", attachment.file_name)
    }

    /// Describe a non-audio/image/video file attachment.
    fn process_file(&self, attachment: &MediaAttachment) -> String {
        let mime = attachment
            .mime_type
            .as_deref()
            .unwrap_or("application/octet-stream");
        let path = std::path::Path::new(&attachment.file_name);

        if path.is_absolute() {
            let display_name = path
                .file_name()
                .and_then(|name| name.to_str())
                .unwrap_or(&attachment.file_name);
            format!(
                "[File: {display_name} attached, saved at {}, {} bytes, MIME: {mime}]",
                path.display(),
                attachment.data.len()
            )
        } else {
            format!(
                "[File: {} attached, {} bytes, MIME: {mime}]",
                attachment.file_name,
                attachment.data.len()
            )
        }
    }
}

fn image_payload_for_vision(attachment: &MediaAttachment) -> (String, Cow<'_, [u8]>) {
    let mime = attachment.mime_type.as_deref().unwrap_or("image/jpeg");

    #[cfg(feature = "image-normalization")]
    if is_webp_attachment(attachment, mime) {
        match webp_to_png(&attachment.data) {
            Ok(png) => return ("image/png".to_string(), Cow::Owned(png)),
            Err(err) => {
                tracing::warn!(
                    file = %attachment.file_name,
                    error = %err,
                    error_key = "media_pipeline_webp_to_png_failed",
                    "Media pipeline: failed to normalize WebP image for vision"
                );
            }
        }
    }

    (mime.to_string(), Cow::Borrowed(&attachment.data))
}

#[cfg(feature = "image-normalization")]
fn is_webp_attachment(attachment: &MediaAttachment, mime: &str) -> bool {
    mime.eq_ignore_ascii_case("image/webp")
        || attachment
            .file_name
            .rsplit_once('.')
            .is_some_and(|(_, ext)| ext.eq_ignore_ascii_case("webp"))
}

#[cfg(feature = "image-normalization")]
fn webp_to_png(data: &[u8]) -> anyhow::Result<Vec<u8>> {
    let image = image::load_from_memory_with_format(data, image::ImageFormat::WebP)?;
    let mut cursor = std::io::Cursor::new(Vec::new());
    image.write_to(&mut cursor, image::ImageFormat::Png)?;
    Ok(cursor.into_inner())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn default_pipeline_config(enabled: bool) -> MediaPipelineConfig {
        MediaPipelineConfig {
            enabled,
            transcribe_audio: true,
            describe_images: true,
            summarize_video: true,
        }
    }

    fn sample_audio() -> MediaAttachment {
        MediaAttachment {
            file_name: "voice.ogg".to_string(),
            data: vec![0u8; 100],
            mime_type: Some("audio/ogg".to_string()),
        }
    }

    fn sample_image() -> MediaAttachment {
        MediaAttachment {
            file_name: "photo.jpg".to_string(),
            data: vec![0u8; 50],
            mime_type: Some("image/jpeg".to_string()),
        }
    }

    fn sample_video() -> MediaAttachment {
        MediaAttachment {
            file_name: "clip.mp4".to_string(),
            data: vec![0u8; 200],
            mime_type: Some("video/mp4".to_string()),
        }
    }

    fn sample_file() -> MediaAttachment {
        MediaAttachment {
            file_name: "/tmp/workspace/whatsapp_files/archive.zip".to_string(),
            data: vec![0u8; 123],
            mime_type: Some("application/zip".to_string()),
        }
    }

    #[test]
    fn media_kind_from_mime() {
        let audio = MediaAttachment {
            file_name: "file".to_string(),
            data: vec![],
            mime_type: Some("audio/ogg".to_string()),
        };
        assert_eq!(audio.kind(), MediaKind::Audio);

        let image = MediaAttachment {
            file_name: "file".to_string(),
            data: vec![],
            mime_type: Some("image/png".to_string()),
        };
        assert_eq!(image.kind(), MediaKind::Image);

        let video = MediaAttachment {
            file_name: "file".to_string(),
            data: vec![],
            mime_type: Some("video/mp4".to_string()),
        };
        assert_eq!(video.kind(), MediaKind::Video);
    }

    #[test]
    fn media_kind_from_extension() {
        let audio = MediaAttachment {
            file_name: "voice.ogg".to_string(),
            data: vec![],
            mime_type: None,
        };
        assert_eq!(audio.kind(), MediaKind::Audio);

        let image = MediaAttachment {
            file_name: "photo.png".to_string(),
            data: vec![],
            mime_type: None,
        };
        assert_eq!(image.kind(), MediaKind::Image);

        let video = MediaAttachment {
            file_name: "clip.mp4".to_string(),
            data: vec![],
            mime_type: None,
        };
        assert_eq!(video.kind(), MediaKind::Video);

        let unknown = MediaAttachment {
            file_name: "data.bin".to_string(),
            data: vec![],
            mime_type: None,
        };
        assert_eq!(unknown.kind(), MediaKind::Unknown);
    }

    #[tokio::test]
    async fn disabled_pipeline_returns_original_text() {
        let config = default_pipeline_config(false);
        let tc = TranscriptionConfig::default();
        let pipeline = MediaPipeline::new(&config, &tc, false, None, None);

        let mut attachments = vec![sample_audio()];
        let result = pipeline.process("hello", &mut attachments).await;
        assert_eq!(result, "hello");
    }

    #[tokio::test]
    async fn empty_attachments_returns_original_text() {
        let config = default_pipeline_config(true);
        let tc = TranscriptionConfig::default();
        let pipeline = MediaPipeline::new(&config, &tc, false, None, None);

        let mut attachments = vec![];
        let result = pipeline.process("hello", &mut attachments).await;
        assert_eq!(result, "hello");
    }

    #[tokio::test]
    async fn image_annotation_with_vision() {
        let config = default_pipeline_config(true);
        let tc = TranscriptionConfig::default();
        let pipeline = MediaPipeline::new(&config, &tc, true, None, None);

        let mut attachments = vec![sample_image()];
        let result = pipeline.process("check this", &mut attachments).await;
        assert!(
            result.contains("[Image: photo.jpg attached, will be processed by vision model]"),
            "expected vision annotation, got: {result}"
        );
        assert!(result.contains("[IMAGE:data:image/jpeg;base64,"));
        assert!(result.contains("check this"));
        assert_eq!(attachments.len(), 1, "native vision should keep attachment");
    }

    #[cfg(feature = "image-normalization")]
    #[tokio::test]
    async fn webp_image_is_normalized_to_png_for_vision() {
        let config = default_pipeline_config(true);
        let tc = TranscriptionConfig::default();
        let pipeline = MediaPipeline::new(&config, &tc, true, None, None);
        let mut cursor = std::io::Cursor::new(Vec::new());
        let webp = image::DynamicImage::ImageRgba8(image::RgbaImage::from_pixel(
            1,
            1,
            image::Rgba([255, 0, 0, 255]),
        ));
        webp.write_to(&mut cursor, image::ImageFormat::WebP)
            .expect("test WebP should encode");

        let sticker = MediaAttachment {
            file_name: "sticker.webp".to_string(),
            data: cursor.into_inner(),
            mime_type: Some("image/webp".to_string()),
        };

        let mut attachments = vec![sticker];
        let result = pipeline.process("what is this?", &mut attachments).await;

        assert!(result.contains("[IMAGE:data:image/png;base64,"));
        assert!(!result.contains("[IMAGE:data:image/webp;base64,"));
        assert!(result.contains("what is this?"));
    }

    #[tokio::test]
    async fn image_annotation_without_vision() {
        let config = default_pipeline_config(true);
        let tc = TranscriptionConfig::default();
        let pipeline = MediaPipeline::new(&config, &tc, false, None, None);

        let mut attachments = vec![sample_image()];
        let result = pipeline.process("check this", &mut attachments).await;
        assert!(
            result.contains("[Image: photo.jpg attached]"),
            "expected basic image annotation, got: {result}"
        );
        assert_eq!(attachments.len(), 1);
    }

    #[tokio::test]
    async fn video_annotation() {
        let config = default_pipeline_config(true);
        let tc = TranscriptionConfig::default();
        let pipeline = MediaPipeline::new(&config, &tc, false, None, None);

        let mut attachments = vec![sample_video()];
        let result = pipeline.process("watch", &mut attachments).await;
        assert!(
            result.contains("[Video: clip.mp4 attached]"),
            "expected video annotation, got: {result}"
        );
    }

    #[tokio::test]
    async fn file_annotation_includes_saved_path_and_size() {
        let config = default_pipeline_config(true);
        let tc = TranscriptionConfig::default();
        let pipeline = MediaPipeline::new(&config, &tc, false, None, None);

        let mut attachments = vec![sample_file()];
        let result = pipeline.process("install this", &mut attachments).await;
        assert!(
            result.contains("[File: archive.zip attached, saved at /tmp/workspace/whatsapp_files/archive.zip, 123 bytes, MIME: application/zip]"),
            "expected saved file annotation, got: {result}"
        );
        assert!(result.contains("install this"));
    }

    #[tokio::test]
    async fn audio_without_transcription_enabled() {
        let config = default_pipeline_config(true);
        let tc = TranscriptionConfig {
            enabled: false,
            ..Default::default()
        };
        let pipeline = MediaPipeline::new(&config, &tc, false, None, None);

        let mut attachments = vec![sample_audio()];
        let result = pipeline.process("", &mut attachments).await;
        assert_eq!(result, "[Audio: attached]");
    }

    #[tokio::test]
    async fn multiple_attachments_produce_multiple_annotations() {
        let config = default_pipeline_config(true);
        let tc = TranscriptionConfig {
            enabled: false,
            ..Default::default()
        };
        let pipeline = MediaPipeline::new(&config, &tc, false, None, None);

        let mut attachments = vec![sample_audio(), sample_image(), sample_video()];
        let result = pipeline.process("context", &mut attachments).await;

        assert!(
            result.contains("[Audio: attached]"),
            "missing audio annotation"
        );
        assert!(
            result.contains("[Image: photo.jpg attached]"),
            "missing image annotation"
        );
        assert!(
            result.contains("[Video: clip.mp4 attached]"),
            "missing video annotation"
        );
        assert!(result.contains("context"), "missing original text");
    }

    #[tokio::test]
    async fn disabled_sub_features_skip_processing() {
        let config = MediaPipelineConfig {
            enabled: true,
            transcribe_audio: false,
            describe_images: false,
            summarize_video: false,
        };
        let tc = TranscriptionConfig::default();
        let pipeline = MediaPipeline::new(&config, &tc, false, None, None);

        let mut attachments = vec![sample_audio(), sample_image(), sample_video()];
        let result = pipeline.process("hello", &mut attachments).await;
        assert_eq!(result, "hello");
    }

    struct MockVisionProvider {
        description: String,
    }

    #[async_trait::async_trait]
    impl Provider for MockVisionProvider {
        async fn chat_with_system(
            &self,
            _system_prompt: Option<&str>,
            _message: &str,
            _model: &str,
            _temperature: Option<f64>,
        ) -> anyhow::Result<String> {
            Ok(self.description.clone())
        }

        async fn chat(
            &self,
            _request: ChatRequest<'_>,
            _model: &str,
            _temperature: Option<f64>,
        ) -> anyhow::Result<zeroclaw_api::provider::ChatResponse> {
            Ok(zeroclaw_api::provider::ChatResponse {
                text: Some(self.description.clone()),
                tool_calls: vec![],
                usage: None,
                reasoning_content: None,
            })
        }
    }

    #[tokio::test]
    async fn image_description_via_vision_provider_transcribes_and_removes_attachment() {
        let config = default_pipeline_config(true);
        let tc = TranscriptionConfig::default();
        let provider = Arc::new(MockVisionProvider {
            description: "A cute orange cat sleeping on a computer keyboard.".to_string(),
        });
        let pipeline = MediaPipeline::new(
            &config,
            &tc,
            false, // Vision not natively available on primary provider
            Some(provider),
            Some("vision-model-xyz".to_string()),
        );

        let mut attachments = vec![sample_image()];
        let result = pipeline
            .process("identify this cat", &mut attachments)
            .await;

        assert!(
            result.contains("[Image description (photo.jpg): A cute orange cat sleeping on a computer keyboard.]"),
            "expected transcription summary annotation, got: {result}"
        );
        assert!(result.contains("identify this cat"));
        assert_eq!(
            attachments.len(),
            0,
            "transcribed attachment should be removed from the list"
        );
    }
}

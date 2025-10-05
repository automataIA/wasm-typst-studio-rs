use typst_as_lib::{typst_kit_options::TypstKitFontOptions, TypstEngine};
use std::collections::HashMap;
use base64::{engine::general_purpose::STANDARD, Engine as _};

/// Wrapper per compilazione Typst con API reale
///
/// NOTA: Per semplicità, ricreiamo l'engine ad ogni compilazione.
/// In futuro potremmo ottimizzare usando un file resolver dinamico.
pub struct TypstCompiler;

impl TypstCompiler {
    /// Crea nuovo compilatore Typst (stateless)
    pub fn new() -> Result<Self, String> {
        Ok(Self)
    }

    /// Compila sorgente Typst e ritorna SVG
    ///
    /// Usa l'API reale di typst-as-lib per compilare il documento,
    /// poi lo converte in SVG usando typst-svg.
    ///
    /// Accetta un dizionario di immagini (image_id -> base64_data)
    /// che saranno iniettate inline quando vengono referenziate.
    ///
    /// Accetta opzionalmente una bibliografia in formato YAML che sarà
    /// registrata come file statico "refs.yml" tramite il file resolver.
    pub fn compile_to_svg(&self, source: &str, bibliography: Option<&str>, images: &HashMap<String, String>) -> Result<String, String> {
        log::info!("Compiling Typst source: {} chars", source.len());

        if source.trim().is_empty() {
            return Err("Source code is empty".to_string());
        }

        log::info!("Processing source for compilation");

        // Prepare static files for the file resolver
        let mut static_files: Vec<(&str, &[u8])> = Vec::new();

        // Add bibliography if provided
        if let Some(bib_content) = bibliography {
            log::info!("Adding bibliography file: {} chars", bib_content.len());
            static_files.push(("refs.yml", bib_content.as_bytes()));
        }

        // Decode base64 images and prepare for static file resolver
        let mut decoded_images: Vec<(String, Vec<u8>)> = Vec::new();

        for (img_id, base64_data) in images {
            // Extract actual image data from data URL (remove "data:image/...;base64," prefix)
            let image_data = if let Some(comma_pos) = base64_data.find(',') {
                &base64_data[comma_pos + 1..]
            } else {
                base64_data
            };

            // Decode base64 to raw bytes
            match STANDARD.decode(image_data) {
                Ok(bytes) => {
                    log::info!("Decoded image {}: {} bytes", img_id, bytes.len());
                    decoded_images.push((img_id.clone(), bytes));
                }
                Err(e) => {
                    log::error!("Failed to decode base64 for {}: {:?}", img_id, e);
                }
            }
        }

        // Add decoded images to static files
        for (img_id, bytes) in &decoded_images {
            static_files.push((img_id.as_str(), bytes.as_slice()));
        }

        // Build engine with static file resolver
        let mut engine_builder = TypstEngine::builder()
            .main_file(source)
            .search_fonts_with(
                TypstKitFontOptions::default()
                    .include_system_fonts(false) // Non disponibili in WASM
                    .include_embedded_fonts(true), // Usa font embedded
            );

        // Register all static files (bibliography + images) with the file resolver
        if !static_files.is_empty() {
            engine_builder = engine_builder.with_static_file_resolver(static_files.clone());
            log::info!("Registered {} static files with file resolver", static_files.len());
        }

        let engine = engine_builder.build();

        // Compila il documento
        let result = engine.compile::<typst::layout::PagedDocument>();

        // Log warnings
        if !result.warnings.is_empty() {
            log::warn!("Compilation warnings: {} warnings", result.warnings.len());
            for warning in &result.warnings {
                log::warn!("  - {:?}", warning);
            }
        }

        // Estrai document e converti a SVG
        match result.output {
            Ok(doc) => {
                log::info!("Document compiled successfully, {} pages", doc.pages.len());

                // Generate individual SVGs for each page and combine them in a container
                let mut combined_svg = String::new();

                for (i, page) in doc.pages.iter().enumerate() {
                    log::info!("Generating SVG for page {}: size = {:?}", i + 1, page.frame.size());

                    // Generate SVG for this single page
                    let page_svg = typst_svg::svg(page);

                    // Wrap each page SVG in a div for separation
                    if i > 0 {
                        combined_svg.push_str("<div style=\"margin-top: 10px; border-top: 1px solid #ccc; padding-top: 10px;\">");
                    } else {
                        combined_svg.push_str("<div>");
                    }
                    combined_svg.push_str(&page_svg);
                    combined_svg.push_str("</div>");
                }

                log::info!("Combined SVG generated for {} pages, {} bytes", doc.pages.len(), combined_svg.len());
                Ok(combined_svg)
            }
            Err(e) => Err(format!("Compilation error: {:?}", e)),
        }
    }

    /// Compila sorgente Typst e ritorna PDF bytes
    ///
    /// Usa l'API reale di typst-as-lib per compilare il documento,
    /// poi lo converte in PDF usando typst-pdf.
    ///
    /// Accetta opzionalmente una bibliografia in formato YAML che sarà
    /// registrata come file statico "refs.yml" tramite il file resolver.
    pub fn compile_to_pdf(&self, source: &str, bibliography: Option<&str>, images: &HashMap<String, String>) -> Result<Vec<u8>, String> {
        log::info!("Compiling Typst source to PDF: {} chars", source.len());

        if source.trim().is_empty() {
            return Err("Source code is empty".to_string());
        }

        log::info!("Processing source for PDF compilation");

        // Prepare static files for the file resolver
        let mut static_files: Vec<(&str, &[u8])> = Vec::new();

        // Add bibliography if provided
        if let Some(bib_content) = bibliography {
            log::info!("Adding bibliography file: {} chars", bib_content.len());
            static_files.push(("refs.yml", bib_content.as_bytes()));
        }

        // Decode base64 images
        let mut decoded_images: Vec<(String, Vec<u8>)> = Vec::new();

        for (img_id, base64_data) in images {
            let image_data = if let Some(comma_pos) = base64_data.find(',') {
                &base64_data[comma_pos + 1..]
            } else {
                base64_data
            };

            match STANDARD.decode(image_data) {
                Ok(bytes) => {
                    log::info!("Decoded image {}: {} bytes", img_id, bytes.len());
                    decoded_images.push((img_id.clone(), bytes));
                }
                Err(e) => {
                    log::error!("Failed to decode base64 for {}: {:?}", img_id, e);
                }
            }
        }

        // Add decoded images to static files
        for (img_id, bytes) in &decoded_images {
            static_files.push((img_id.as_str(), bytes.as_slice()));
        }

        // Build engine
        let mut engine_builder = TypstEngine::builder()
            .main_file(source)
            .search_fonts_with(
                TypstKitFontOptions::default()
                    .include_system_fonts(false)
                    .include_embedded_fonts(true),
            );

        // Register all static files (bibliography + images) with the file resolver
        if !static_files.is_empty() {
            engine_builder = engine_builder.with_static_file_resolver(static_files.clone());
            log::info!("Registered {} static files with file resolver", static_files.len());
        }

        let engine = engine_builder.build();

        // Compila il documento
        let result = engine.compile::<typst::layout::PagedDocument>();

        // Log warnings
        if !result.warnings.is_empty() {
            log::warn!("Compilation warnings: {} warnings", result.warnings.len());
            for warning in &result.warnings {
                log::warn!("  - {:?}", warning);
            }
        }

        // Estrai document e converti a PDF
        match result.output {
            Ok(doc) => {
                log::info!("Document compiled successfully, {} pages", doc.pages.len());

                // Converti a PDF usando typst_pdf::pdf
                let pdf_options = typst_pdf::PdfOptions::default();
                let pdf_result = typst_pdf::pdf(&doc, &pdf_options);

                match pdf_result {
                    Ok(pdf_bytes) => {
                        log::info!("PDF generated, {} bytes", pdf_bytes.len());
                        Ok(pdf_bytes)
                    }
                    Err(e) => Err(format!("PDF generation error: {:?}", e)),
                }
            }
            Err(e) => Err(format!("Compilation error: {:?}", e)),
        }
    }
}

impl Default for TypstCompiler {
    fn default() -> Self {
        Self::new().expect("Failed to create TypstCompiler")
    }
}

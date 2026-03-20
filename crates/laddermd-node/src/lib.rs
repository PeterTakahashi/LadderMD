use laddermd_core::{parser, renderer::MarkdownRenderer, validator, writer};
use napi::bindgen_prelude::*;
use napi_derive::napi;

/// Convert PLCopen XML to Markdown.
#[napi]
pub fn convert_xml(xml: String) -> Result<String> {
    let project = parser::parse(&xml)
        .map_err(|e| Error::from_reason(format!("{e}")))?;
    let renderer = MarkdownRenderer::default();
    Ok(renderer.render(&project))
}

/// Convert Mitsubishi mnemonic format to Markdown.
#[napi]
pub fn convert_mnemonic(mnemonic: String) -> Result<String> {
    let project = parser::parse_mnemonic(&mnemonic, None, None)
        .map_err(|e| Error::from_reason(format!("{e}")))?;
    let renderer = MarkdownRenderer::default();
    Ok(renderer.render(&project))
}

/// Convert Mitsubishi mnemonic to PLCopen XML.
#[napi]
pub fn mnemonic_to_xml(mnemonic: String) -> Result<String> {
    let project = parser::parse_mnemonic(&mnemonic, None, None)
        .map_err(|e| Error::from_reason(format!("{e}")))?;
    writer::write(&project).map_err(|e| Error::from_reason(format!("{e}")))
}

/// Convert PLCopen XML to Mitsubishi mnemonic format.
#[napi]
pub fn xml_to_mnemonic(xml: String) -> Result<String> {
    let project = parser::parse(&xml)
        .map_err(|e| Error::from_reason(format!("{e}")))?;
    Ok(parser::write_mnemonic(&project))
}

/// Result of roundtrip validation.
#[napi(object)]
pub struct ValidationResult {
    pub parse_ok: bool,
    pub roundtrip_ok: bool,
    pub total_rungs: u32,
    pub contacts: u32,
    pub coils: u32,
    pub blocks: u32,
    pub errors: Vec<String>,
}

/// Validate roundtrip conversion of PLCopen XML.
#[napi]
pub fn validate(xml: String) -> Result<ValidationResult> {
    let result = validator::validate(&xml)
        .map_err(|e| Error::from_reason(format!("{e}")))?;

    Ok(ValidationResult {
        parse_ok: result.parse_ok,
        roundtrip_ok: result.roundtrip_ok,
        total_rungs: result.total_rungs as u32,
        contacts: result.contacts,
        coils: result.coils,
        blocks: result.blocks,
        errors: result.errors,
    })
}

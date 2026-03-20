#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

use laddermd_core::{parser, renderer::MarkdownRenderer, validator, writer};
use serde::{Deserialize, Serialize};

#[derive(Serialize)]
struct ConvertResult {
    markdown: String,
    project_name: String,
    rung_count: usize,
}

#[derive(Serialize)]
struct ValidateResult {
    parse_ok: bool,
    roundtrip_ok: bool,
    total_rungs: usize,
    contacts: u32,
    coils: u32,
    blocks: u32,
    errors: Vec<String>,
}

#[tauri::command]
fn convert_xml(xml: String) -> Result<ConvertResult, String> {
    let project = parser::parse(&xml).map_err(|e| format!("{e}"))?;
    let renderer = MarkdownRenderer::default();
    let markdown = renderer.render(&project);
    let rung_count: usize = project.programs.iter().map(|p| p.rungs.len()).sum();
    Ok(ConvertResult {
        markdown,
        project_name: project.name.clone(),
        rung_count,
    })
}

#[tauri::command]
fn convert_mnemonic(mnemonic: String) -> Result<ConvertResult, String> {
    let project =
        parser::parse_mnemonic(&mnemonic, None, None).map_err(|e| format!("{e}"))?;
    let renderer = MarkdownRenderer::default();
    let markdown = renderer.render(&project);
    let rung_count: usize = project.programs.iter().map(|p| p.rungs.len()).sum();
    Ok(ConvertResult {
        markdown,
        project_name: project.name.clone(),
        rung_count,
    })
}

#[tauri::command]
fn validate_xml(xml: String) -> Result<ValidateResult, String> {
    let result = validator::validate(&xml).map_err(|e| format!("{e}"))?;
    Ok(ValidateResult {
        parse_ok: result.parse_ok,
        roundtrip_ok: result.roundtrip_ok,
        total_rungs: result.total_rungs,
        contacts: result.contacts,
        coils: result.coils,
        blocks: result.blocks,
        errors: result.errors,
    })
}

#[tauri::command]
fn xml_to_mnemonic(xml: String) -> Result<String, String> {
    let project = parser::parse(&xml).map_err(|e| format!("{e}"))?;
    Ok(parser::write_mnemonic(&project))
}

#[tauri::command]
fn mnemonic_to_xml(mnemonic: String) -> Result<String, String> {
    let project =
        parser::parse_mnemonic(&mnemonic, None, None).map_err(|e| format!("{e}"))?;
    writer::write(&project).map_err(|e| format!("{e}"))
}

fn main() {
    tauri::Builder::default()
        .invoke_handler(tauri::generate_handler![
            convert_xml,
            convert_mnemonic,
            validate_xml,
            xml_to_mnemonic,
            mnemonic_to_xml,
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}

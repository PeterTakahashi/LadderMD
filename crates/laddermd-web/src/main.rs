use axum::{
    extract::Json,
    http::StatusCode,
    response::IntoResponse,
    routing::{get, post},
    Router,
};
use laddermd_core::{model::Project, parser, renderer::MarkdownRenderer, validator, writer};
use serde::{Deserialize, Serialize};
use tower_http::cors::CorsLayer;

#[derive(Deserialize)]
struct ConvertRequest {
    /// Input content (XML or mnemonic text)
    input: String,
    /// Input format: "xml" or "mnemonic"
    #[serde(default = "default_format")]
    format: String,
    /// Output options
    #[serde(default)]
    no_diagram: bool,
    #[serde(default)]
    no_table: bool,
    #[serde(default)]
    no_logic: bool,
}

fn default_format() -> String {
    "xml".to_string()
}

#[derive(Serialize)]
struct ConvertResponse {
    markdown: String,
    project_name: String,
    program_count: usize,
    rung_count: usize,
}

#[derive(Serialize)]
struct ErrorResponse {
    error: String,
}

#[derive(Deserialize)]
struct ValidateRequest {
    xml: String,
}

#[derive(Serialize)]
struct ValidateResponse {
    parse_ok: bool,
    roundtrip_ok: bool,
    total_rungs: usize,
    contacts: u32,
    coils: u32,
    blocks: u32,
    errors: Vec<String>,
}

#[derive(Deserialize)]
struct Mn2XmlRequest {
    mnemonic: String,
}

#[derive(Deserialize)]
struct Xml2MnRequest {
    xml: String,
}

#[derive(Serialize)]
struct TextResponse {
    output: String,
}

#[derive(Serialize)]
struct InfoResponse {
    project_name: String,
    programs: Vec<ProgramInfo>,
}

#[derive(Serialize)]
struct ProgramInfo {
    name: String,
    rungs: usize,
    contacts: u32,
    coils: u32,
    blocks: u32,
}

fn parse_project(input: &str, format: &str) -> Result<Project, String> {
    match format {
        "mnemonic" | "mn" => parser::parse_mnemonic(input, None, None)
            .map_err(|e| format!("Mnemonic parse error: {e}")),
        _ => parser::parse(input).map_err(|e| format!("XML parse error: {e}")),
    }
}

async fn health() -> &'static str {
    "ok"
}

async fn convert(Json(req): Json<ConvertRequest>) -> impl IntoResponse {
    let project = match parse_project(&req.input, &req.format) {
        Ok(p) => p,
        Err(e) => {
            return (
                StatusCode::BAD_REQUEST,
                Json(serde_json::json!({"error": e})),
            )
                .into_response()
        }
    };

    let renderer = MarkdownRenderer {
        render_diagram: !req.no_diagram,
        render_device_table: !req.no_table,
        render_logic: !req.no_logic,
    };

    let markdown = renderer.render(&project);
    let rung_count: usize = project.programs.iter().map(|p| p.rungs.len()).sum();

    Json(ConvertResponse {
        markdown,
        project_name: project.name.clone(),
        program_count: project.programs.len(),
        rung_count,
    })
    .into_response()
}

async fn validate(Json(req): Json<ValidateRequest>) -> impl IntoResponse {
    let result = match validator::validate(&req.xml) {
        Ok(r) => r,
        Err(e) => {
            return (
                StatusCode::BAD_REQUEST,
                Json(serde_json::json!({"error": format!("{e}")})),
            )
                .into_response()
        }
    };

    Json(ValidateResponse {
        parse_ok: result.parse_ok,
        roundtrip_ok: result.roundtrip_ok,
        total_rungs: result.total_rungs,
        contacts: result.contacts,
        coils: result.coils,
        blocks: result.blocks,
        errors: result.errors,
    })
    .into_response()
}

async fn mn2xml(Json(req): Json<Mn2XmlRequest>) -> impl IntoResponse {
    let project = match parser::parse_mnemonic(&req.mnemonic, None, None) {
        Ok(p) => p,
        Err(e) => {
            return (
                StatusCode::BAD_REQUEST,
                Json(serde_json::json!({"error": format!("{e}")})),
            )
                .into_response()
        }
    };

    match writer::write(&project) {
        Ok(xml) => Json(TextResponse { output: xml }).into_response(),
        Err(e) => (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(serde_json::json!({"error": format!("{e}")})),
        )
            .into_response(),
    }
}

async fn xml2mn(Json(req): Json<Xml2MnRequest>) -> impl IntoResponse {
    let project = match parser::parse(&req.xml) {
        Ok(p) => p,
        Err(e) => {
            return (
                StatusCode::BAD_REQUEST,
                Json(serde_json::json!({"error": format!("{e}")})),
            )
                .into_response()
        }
    };

    let mn = parser::write_mnemonic(&project);
    Json(TextResponse { output: mn }).into_response()
}

async fn info(Json(req): Json<ConvertRequest>) -> impl IntoResponse {
    let project = match parse_project(&req.input, &req.format) {
        Ok(p) => p,
        Err(e) => {
            return (
                StatusCode::BAD_REQUEST,
                Json(serde_json::json!({"error": e})),
            )
                .into_response()
        }
    };

    let programs: Vec<ProgramInfo> = project
        .programs
        .iter()
        .map(|prog| {
            let (mut contacts, mut coils, mut blocks) = (0u32, 0u32, 0u32);
            for rung in &prog.rungs {
                for elem in &rung.elements {
                    match elem {
                        laddermd_core::model::RungElement::Contact(_) => contacts += 1,
                        laddermd_core::model::RungElement::Coil(_) => coils += 1,
                        laddermd_core::model::RungElement::Block(_) => blocks += 1,
                    }
                }
            }
            ProgramInfo {
                name: prog.name.clone(),
                rungs: prog.rungs.len(),
                contacts,
                coils,
                blocks,
            }
        })
        .collect();

    Json(InfoResponse {
        project_name: project.name.clone(),
        programs,
    })
    .into_response()
}

#[tokio::main]
async fn main() {
    let app = Router::new()
        .route("/health", get(health))
        .route("/api/convert", post(convert))
        .route("/api/validate", post(validate))
        .route("/api/mn2xml", post(mn2xml))
        .route("/api/xml2mn", post(xml2mn))
        .route("/api/info", post(info))
        .layer(CorsLayer::permissive());

    let addr = "0.0.0.0:3000";
    eprintln!("LadderMD Web API listening on http://{addr}");
    let listener = tokio::net::TcpListener::bind(addr).await.unwrap();
    axum::serve(listener, app).await.unwrap();
}

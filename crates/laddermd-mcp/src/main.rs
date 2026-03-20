//! LadderMD MCP (Model Context Protocol) Server
//!
//! Implements the MCP stdio transport, exposing laddermd tools to AI agents.
//! Run with: `laddermd-mcp` (communicates via stdin/stdout JSON-RPC).

use laddermd_core::{parser, renderer::MarkdownRenderer, validator, writer};
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use std::io::{self, BufRead, Write};

// ── JSON-RPC types ───────────────────────────────────────────────────

#[derive(Deserialize)]
struct JsonRpcRequest {
    #[allow(dead_code)]
    jsonrpc: String,
    id: Option<Value>,
    method: String,
    #[serde(default)]
    params: Value,
}

#[derive(Serialize)]
struct JsonRpcResponse {
    jsonrpc: String,
    id: Value,
    #[serde(skip_serializing_if = "Option::is_none")]
    result: Option<Value>,
    #[serde(skip_serializing_if = "Option::is_none")]
    error: Option<JsonRpcError>,
}

#[derive(Serialize)]
struct JsonRpcError {
    code: i64,
    message: String,
}

impl JsonRpcResponse {
    fn success(id: Value, result: Value) -> Self {
        Self {
            jsonrpc: "2.0".to_string(),
            id,
            result: Some(result),
            error: None,
        }
    }

    fn error(id: Value, code: i64, message: String) -> Self {
        Self {
            jsonrpc: "2.0".to_string(),
            id,
            result: None,
            error: Some(JsonRpcError { code, message }),
        }
    }
}

// ── MCP Protocol ─────────────────────────────────────────────────────

const SERVER_NAME: &str = "laddermd";
const SERVER_VERSION: &str = env!("CARGO_PKG_VERSION");

fn handle_initialize(_params: &Value) -> Value {
    json!({
        "protocolVersion": "2024-11-05",
        "capabilities": {
            "tools": {}
        },
        "serverInfo": {
            "name": SERVER_NAME,
            "version": SERVER_VERSION
        }
    })
}

fn tool_definitions() -> Value {
    json!({
        "tools": [
            {
                "name": "convert_xml_to_markdown",
                "description": "Convert a PLCopen XML ladder diagram to human-readable Markdown. Returns logic expressions, device tables, and ASCII ladder diagrams.",
                "inputSchema": {
                    "type": "object",
                    "properties": {
                        "xml": {
                            "type": "string",
                            "description": "PLCopen XML content"
                        }
                    },
                    "required": ["xml"]
                }
            },
            {
                "name": "convert_mnemonic_to_markdown",
                "description": "Convert Mitsubishi MELSEC mnemonic (instruction list) format to Markdown. Supports LD, AND, OR, OUT, SET, RST, MPS/MRD/MPP, ORB, ANB, timer/counter instructions.",
                "inputSchema": {
                    "type": "object",
                    "properties": {
                        "mnemonic": {
                            "type": "string",
                            "description": "Mitsubishi mnemonic program text"
                        }
                    },
                    "required": ["mnemonic"]
                }
            },
            {
                "name": "convert_mnemonic_to_xml",
                "description": "Convert Mitsubishi MELSEC mnemonic format to PLCopen XML.",
                "inputSchema": {
                    "type": "object",
                    "properties": {
                        "mnemonic": {
                            "type": "string",
                            "description": "Mitsubishi mnemonic program text"
                        }
                    },
                    "required": ["mnemonic"]
                }
            },
            {
                "name": "convert_xml_to_mnemonic",
                "description": "Convert PLCopen XML ladder diagram to Mitsubishi MELSEC mnemonic format.",
                "inputSchema": {
                    "type": "object",
                    "properties": {
                        "xml": {
                            "type": "string",
                            "description": "PLCopen XML content"
                        }
                    },
                    "required": ["xml"]
                }
            },
            {
                "name": "validate_roundtrip",
                "description": "Validate roundtrip conversion of a PLCopen XML ladder diagram. Parses XML to internal model, writes back to XML, re-parses, and checks logical equivalence.",
                "inputSchema": {
                    "type": "object",
                    "properties": {
                        "xml": {
                            "type": "string",
                            "description": "PLCopen XML content"
                        }
                    },
                    "required": ["xml"]
                }
            },
            {
                "name": "get_info",
                "description": "Get summary information about a PLC ladder diagram: project name, program names, rung count, device counts.",
                "inputSchema": {
                    "type": "object",
                    "properties": {
                        "input": {
                            "type": "string",
                            "description": "PLCopen XML or mnemonic content"
                        },
                        "format": {
                            "type": "string",
                            "enum": ["xml", "mnemonic"],
                            "description": "Input format",
                            "default": "xml"
                        }
                    },
                    "required": ["input"]
                }
            }
        ]
    })
}

fn handle_tools_list(_params: &Value) -> Value {
    tool_definitions()
}

fn handle_tools_call(params: &Value) -> Value {
    let name = params.get("name").and_then(|v| v.as_str()).unwrap_or("");
    let arguments = params.get("arguments").cloned().unwrap_or(json!({}));

    let result = match name {
        "convert_xml_to_markdown" => tool_convert_xml(&arguments),
        "convert_mnemonic_to_markdown" => tool_convert_mnemonic(&arguments),
        "convert_mnemonic_to_xml" => tool_mn2xml(&arguments),
        "convert_xml_to_mnemonic" => tool_xml2mn(&arguments),
        "validate_roundtrip" => tool_validate(&arguments),
        "get_info" => tool_info(&arguments),
        _ => Err(format!("Unknown tool: {name}")),
    };

    match result {
        Ok(text) => json!({
            "content": [{
                "type": "text",
                "text": text
            }]
        }),
        Err(e) => json!({
            "content": [{
                "type": "text",
                "text": format!("Error: {e}")
            }],
            "isError": true
        }),
    }
}

// ── Tool implementations ─────────────────────────────────────────────

fn tool_convert_xml(args: &Value) -> Result<String, String> {
    let xml = args
        .get("xml")
        .and_then(|v| v.as_str())
        .ok_or("Missing 'xml' parameter")?;
    let project = parser::parse(xml).map_err(|e| format!("{e}"))?;
    let renderer = MarkdownRenderer::default();
    Ok(renderer.render(&project))
}

fn tool_convert_mnemonic(args: &Value) -> Result<String, String> {
    let mn = args
        .get("mnemonic")
        .and_then(|v| v.as_str())
        .ok_or("Missing 'mnemonic' parameter")?;
    let project = parser::parse_mnemonic(mn, None, None).map_err(|e| format!("{e}"))?;
    let renderer = MarkdownRenderer::default();
    Ok(renderer.render(&project))
}

fn tool_mn2xml(args: &Value) -> Result<String, String> {
    let mn = args
        .get("mnemonic")
        .and_then(|v| v.as_str())
        .ok_or("Missing 'mnemonic' parameter")?;
    let project = parser::parse_mnemonic(mn, None, None).map_err(|e| format!("{e}"))?;
    writer::write(&project).map_err(|e| format!("{e}"))
}

fn tool_xml2mn(args: &Value) -> Result<String, String> {
    let xml = args
        .get("xml")
        .and_then(|v| v.as_str())
        .ok_or("Missing 'xml' parameter")?;
    let project = parser::parse(xml).map_err(|e| format!("{e}"))?;
    Ok(parser::write_mnemonic(&project))
}

fn tool_validate(args: &Value) -> Result<String, String> {
    let xml = args
        .get("xml")
        .and_then(|v| v.as_str())
        .ok_or("Missing 'xml' parameter")?;
    let result = validator::validate(xml).map_err(|e| format!("{e}"))?;

    let mut output = String::new();
    output.push_str(&format!("Parse OK: {} rungs found\n", result.total_rungs));
    output.push_str(&format!(
        "Devices: {} contacts, {} coils, {} blocks\n",
        result.contacts, result.coils, result.blocks
    ));
    if result.roundtrip_ok {
        output.push_str("Roundtrip OK: all rungs logically equivalent\n");
    } else {
        output.push_str("Roundtrip FAILED:\n");
        for err in &result.errors {
            output.push_str(&format!("  - {err}\n"));
        }
    }
    Ok(output)
}

fn tool_info(args: &Value) -> Result<String, String> {
    let input = args
        .get("input")
        .and_then(|v| v.as_str())
        .ok_or("Missing 'input' parameter")?;
    let format = args
        .get("format")
        .and_then(|v| v.as_str())
        .unwrap_or("xml");

    let project = match format {
        "mnemonic" | "mn" => {
            parser::parse_mnemonic(input, None, None).map_err(|e| format!("{e}"))?
        }
        _ => parser::parse(input).map_err(|e| format!("{e}"))?,
    };

    let mut output = format!("Project: {}\n", project.name);
    for prog in &project.programs {
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
        output.push_str(&format!(
            "  Program: {} ({} rungs, {} contacts, {} coils, {} blocks)\n",
            prog.name,
            prog.rungs.len(),
            contacts,
            coils,
            blocks
        ));
    }
    Ok(output)
}

// ── Main loop ────────────────────────────────────────────────────────

fn handle_request(req: &JsonRpcRequest) -> Option<JsonRpcResponse> {
    let id = req.id.clone().unwrap_or(Value::Null);

    let result = match req.method.as_str() {
        "initialize" => handle_initialize(&req.params),
        "notifications/initialized" => return None, // notification, no response
        "tools/list" => handle_tools_list(&req.params),
        "tools/call" => handle_tools_call(&req.params),
        _ => {
            return Some(JsonRpcResponse::error(
                id,
                -32601,
                format!("Method not found: {}", req.method),
            ))
        }
    };

    Some(JsonRpcResponse::success(id, result))
}

fn main() {
    let stdin = io::stdin();
    let mut stdout = io::stdout();

    for line in stdin.lock().lines() {
        let line = match line {
            Ok(l) => l,
            Err(_) => break,
        };

        if line.trim().is_empty() {
            continue;
        }

        let req: JsonRpcRequest = match serde_json::from_str(&line) {
            Ok(r) => r,
            Err(e) => {
                let resp = JsonRpcResponse::error(
                    Value::Null,
                    -32700,
                    format!("Parse error: {e}"),
                );
                let _ = writeln!(stdout, "{}", serde_json::to_string(&resp).unwrap());
                let _ = stdout.flush();
                continue;
            }
        };

        if let Some(resp) = handle_request(&req) {
            let _ = writeln!(stdout, "{}", serde_json::to_string(&resp).unwrap());
            let _ = stdout.flush();
        }
    }
}

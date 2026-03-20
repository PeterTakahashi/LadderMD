//! Mitsubishi MELSEC mnemonic (instruction list) parser.
//!
//! Parses the text-based mnemonic format used by Mitsubishi GX Works / GX Developer.
//!
//! # Format Overview
//!
//! Each line is one instruction:
//! ```text
//! LD   X001      ; Start rung, NO contact
//! AND  X002      ; Series NO contact
//! OR   Y001      ; Parallel NO contact
//! OUT  Y001      ; Output coil
//! ```
//!
//! ## Instructions
//!
//! | Instruction | Meaning                          |
//! |-------------|----------------------------------|
//! | LD          | Load (start new series, NO)       |
//! | LDI         | Load Inverse (start new series, NC) |
//! | AND         | Series AND, NO contact            |
//! | ANI         | Series AND, NC contact            |
//! | OR          | Parallel OR, NO contact           |
//! | ORI         | Parallel OR, NC contact           |
//! | OUT         | Output coil                       |
//! | SET         | Set (latch) coil                  |
//! | RST         | Reset coil                        |
//! | MPS         | Push current result to stack      |
//! | MRD         | Read (peek) stack top             |
//! | MPP         | Pop stack top                     |
//! | ORB         | OR Block (combine parallel paths) |
//! | ANB         | AND Block (combine series blocks) |
//! | OUT T       | Timer output (e.g., OUT T0 K100)  |
//! | OUT C       | Counter output (e.g., OUT C0 K10) |
//! | END         | End of program                    |

use crate::model::*;

/// Errors during mnemonic parsing.
#[derive(Debug, thiserror::Error)]
pub enum MnemonicParseError {
    #[error("line {line}: {message}")]
    Syntax { line: usize, message: String },

    #[error("line {line}: unexpected instruction '{instruction}' - {message}")]
    UnexpectedInstruction {
        line: usize,
        instruction: String,
        message: String,
    },

    #[error("line {line}: missing operand for '{instruction}'")]
    MissingOperand { line: usize, instruction: String },

    #[error("empty program")]
    EmptyProgram,
}

/// Parse a Mitsubishi mnemonic program text into our internal model.
///
/// The optional `program_name` defaults to "Main" if not provided.
pub fn parse_mnemonic(
    input: &str,
    project_name: Option<&str>,
    program_name: Option<&str>,
) -> Result<Project, MnemonicParseError> {
    let project_name = project_name.unwrap_or("MnemonicProject");
    let program_name = program_name.unwrap_or("Main");

    let tokens = tokenize(input)?;

    if tokens.is_empty() {
        return Err(MnemonicParseError::EmptyProgram);
    }

    let rungs = build_rungs_from_tokens(&tokens)?;

    Ok(Project {
        name: project_name.to_string(),
        programs: vec![Program {
            name: program_name.to_string(),
            rungs,
        }],
    })
}

// ── Tokenizer ────────────────────────────────────────────────────────

#[derive(Debug, Clone)]
struct Token {
    line: usize,
    instruction: Instruction,
}

#[derive(Debug, Clone)]
enum Instruction {
    Ld(String),        // LD device
    Ldi(String),       // LDI device
    And(String),       // AND device
    Ani(String),       // ANI device
    Or(String),        // OR device
    Ori(String),       // ORI device
    Out(String),       // OUT device
    Set(String),       // SET device
    Rst(String),       // RST device
    Mps,               // MPS
    Mrd,               // MRD
    Mpp,               // MPP
    Orb,               // ORB
    Anb,               // ANB
    OutTimer(String, String), // OUT Txxx Kyyy
    OutCounter(String, String), // OUT Cxxx Kyyy
    End,               // END
}

fn tokenize(input: &str) -> Result<Vec<Token>, MnemonicParseError> {
    let mut tokens = Vec::new();

    for (line_idx, raw_line) in input.lines().enumerate() {
        let line_num = line_idx + 1;

        // Strip comments (semicolon)
        let line = raw_line.split(';').next().unwrap_or("").trim();
        if line.is_empty() {
            continue;
        }

        let parts: Vec<&str> = line.split_whitespace().collect();
        if parts.is_empty() {
            continue;
        }

        let instr_str = parts[0].to_uppercase();
        let instruction = match instr_str.as_str() {
            "LD" => {
                let dev = require_operand(&parts, line_num, "LD")?;
                Instruction::Ld(dev)
            }
            "LDI" => {
                let dev = require_operand(&parts, line_num, "LDI")?;
                Instruction::Ldi(dev)
            }
            "AND" => {
                let dev = require_operand(&parts, line_num, "AND")?;
                Instruction::And(dev)
            }
            "ANI" => {
                let dev = require_operand(&parts, line_num, "ANI")?;
                Instruction::Ani(dev)
            }
            "OR" => {
                let dev = require_operand(&parts, line_num, "OR")?;
                Instruction::Or(dev)
            }
            "ORI" => {
                let dev = require_operand(&parts, line_num, "ORI")?;
                Instruction::Ori(dev)
            }
            "OUT" => {
                if parts.len() < 2 {
                    return Err(MnemonicParseError::MissingOperand {
                        line: line_num,
                        instruction: "OUT".to_string(),
                    });
                }
                let dev = parts[1].to_uppercase();
                // Check for timer/counter: OUT T0 K100, OUT C0 K10
                if dev.starts_with('T') && dev.len() > 1 && dev[1..].chars().all(|c| c.is_ascii_digit()) {
                    let preset = if parts.len() >= 3 {
                        parts[2].to_uppercase()
                    } else {
                        "K0".to_string()
                    };
                    Instruction::OutTimer(dev, preset)
                } else if dev.starts_with('C') && dev.len() > 1 && dev[1..].chars().all(|c| c.is_ascii_digit()) {
                    let preset = if parts.len() >= 3 {
                        parts[2].to_uppercase()
                    } else {
                        "K0".to_string()
                    };
                    Instruction::OutCounter(dev, preset)
                } else {
                    Instruction::Out(dev)
                }
            }
            "SET" => {
                let dev = require_operand(&parts, line_num, "SET")?;
                Instruction::Set(dev)
            }
            "RST" => {
                let dev = require_operand(&parts, line_num, "RST")?;
                Instruction::Rst(dev)
            }
            "MPS" => Instruction::Mps,
            "MRD" => Instruction::Mrd,
            "MPP" => Instruction::Mpp,
            "ORB" => Instruction::Orb,
            "ANB" => Instruction::Anb,
            "END" => Instruction::End,
            other => {
                return Err(MnemonicParseError::Syntax {
                    line: line_num,
                    message: format!("unknown instruction: {other}"),
                });
            }
        };

        tokens.push(Token {
            line: line_num,
            instruction,
        });
    }

    Ok(tokens)
}

fn require_operand(
    parts: &[&str],
    line: usize,
    instruction: &str,
) -> Result<String, MnemonicParseError> {
    if parts.len() < 2 {
        return Err(MnemonicParseError::MissingOperand {
            line,
            instruction: instruction.to_string(),
        });
    }
    Ok(parts[1].to_uppercase())
}

// ── Rung Builder ─────────────────────────────────────────────────────
//
// Mitsubishi mnemonic uses a stack-based evaluation model.
// Each LD/LDI starts a new "operand" on the logical stack.
// AND/ANI narrow the current operand (series connection).
// OR/ORI widen the current operand (parallel connection).
// ORB/ANB combine stack entries.
// MPS/MRD/MPP handle branching points.
// OUT/SET/RST emit output coils from the current result.

fn build_rungs_from_tokens(tokens: &[Token]) -> Result<Vec<Rung>, MnemonicParseError> {
    let mut rungs: Vec<Rung> = Vec::new();
    let mut next_id: u32 = 1;

    // We group tokens into rungs. A rung starts at an LD/LDI and ends
    // when the next LD/LDI appears (at stack depth 0) or at END.
    let mut rung_groups: Vec<Vec<&Token>> = Vec::new();
    let mut current_group: Vec<&Token> = Vec::new();
    let mut ld_depth: i32 = 0;

    for token in tokens {
        match &token.instruction {
            Instruction::End => break,
            Instruction::Ld(_) | Instruction::Ldi(_) => {
                if current_group.is_empty() {
                    // First LD in a potential rung
                    current_group.push(token);
                    ld_depth = 1;
                } else {
                    // Another LD could be a nested LD for ORB/ANB, or a new rung.
                    // We track: if the previous group ended with outputs and this is
                    // a fresh LD at depth 0, start a new rung.
                    let has_output = current_group.iter().any(|t| {
                        matches!(
                            t.instruction,
                            Instruction::Out(_)
                                | Instruction::Set(_)
                                | Instruction::Rst(_)
                                | Instruction::OutTimer(_, _)
                                | Instruction::OutCounter(_, _)
                        )
                    });

                    // Check if the next tokens after this LD will have ORB/ANB
                    // If so, this LD is nested, not a new rung.
                    // Simple heuristic: if ld_depth > 0 and no output yet, it's nested.
                    if has_output && ld_depth <= 1 {
                        // Previous rung is complete, start new one
                        rung_groups.push(current_group);
                        current_group = vec![token];
                        ld_depth = 1;
                    } else {
                        current_group.push(token);
                        ld_depth += 1;
                    }
                }
            }
            Instruction::Orb | Instruction::Anb => {
                current_group.push(token);
                ld_depth = (ld_depth - 1).max(1);
            }
            _ => {
                current_group.push(token);
            }
        }
    }

    if !current_group.is_empty() {
        rung_groups.push(current_group);
    }

    // Convert each group to a Rung
    for group in &rung_groups {
        let rung = build_single_rung(group, &mut next_id)?;
        rungs.push(rung);
    }

    Ok(rungs)
}

/// Represents a node in the logical evaluation stack.
#[derive(Debug, Clone)]
struct LogicNode {
    /// The elements accumulated in this logic path.
    elements: Vec<RungElement>,
    /// Connections between elements (from_id, to_id).
    connections: Vec<Connection>,
    /// The localId of the last element in the series chain (for connecting next).
    last_id: Option<u32>,
}

fn build_single_rung(
    tokens: &[&Token],
    next_id: &mut u32,
) -> Result<Rung, MnemonicParseError> {
    let mut stack: Vec<LogicNode> = Vec::new();
    let mut branch_stack: Vec<u32> = Vec::new(); // MPS/MRD/MPP stack of last_ids

    // All elements and connections for this rung
    let mut all_elements: Vec<RungElement> = Vec::new();
    let mut all_connections: Vec<Connection> = Vec::new();

    for token in tokens {
        match &token.instruction {
            Instruction::Ld(dev) => {
                let id = alloc_id(next_id);
                let node = LogicNode {
                    elements: vec![RungElement::Contact(Contact {
                        variable: dev.clone(),
                        contact_type: ContactType::NormallyOpen,
                        local_id: id,
                    })],
                    connections: vec![],
                    last_id: Some(id),
                };
                stack.push(node);
            }
            Instruction::Ldi(dev) => {
                let id = alloc_id(next_id);
                let node = LogicNode {
                    elements: vec![RungElement::Contact(Contact {
                        variable: dev.clone(),
                        contact_type: ContactType::NormallyClosed,
                        local_id: id,
                    })],
                    connections: vec![],
                    last_id: Some(id),
                };
                stack.push(node);
            }
            Instruction::And(dev) => {
                let id = alloc_id(next_id);
                let top = stack.last_mut().ok_or(MnemonicParseError::Syntax {
                    line: token.line,
                    message: "AND without prior LD".to_string(),
                })?;
                if let Some(prev_id) = top.last_id {
                    top.connections.push(Connection {
                        from_id: prev_id,
                        to_id: id,
                    });
                }
                top.elements.push(RungElement::Contact(Contact {
                    variable: dev.clone(),
                    contact_type: ContactType::NormallyOpen,
                    local_id: id,
                }));
                top.last_id = Some(id);
            }
            Instruction::Ani(dev) => {
                let id = alloc_id(next_id);
                let top = stack.last_mut().ok_or(MnemonicParseError::Syntax {
                    line: token.line,
                    message: "ANI without prior LD".to_string(),
                })?;
                if let Some(prev_id) = top.last_id {
                    top.connections.push(Connection {
                        from_id: prev_id,
                        to_id: id,
                    });
                }
                top.elements.push(RungElement::Contact(Contact {
                    variable: dev.clone(),
                    contact_type: ContactType::NormallyClosed,
                    local_id: id,
                }));
                top.last_id = Some(id);
            }
            Instruction::Or(dev) => {
                // OR adds a parallel NO contact to the top stack entry.
                // The parallel contact connects from the same source as the first
                // element in the current node, and its output merges with last_id.
                let id = alloc_id(next_id);
                let top = stack.last_mut().ok_or(MnemonicParseError::Syntax {
                    line: token.line,
                    message: "OR without prior LD".to_string(),
                })?;
                top.elements.push(RungElement::Contact(Contact {
                    variable: dev.clone(),
                    contact_type: ContactType::NormallyOpen,
                    local_id: id,
                }));
                // The OR contact is parallel - it doesn't connect to last_id,
                // it represents an alternative path. We keep last_id as-is since
                // downstream elements connect to both paths (handled at output).
                // Store the OR contact's id alongside last_id for output connections.
                top.last_id = Some(id);
            }
            Instruction::Ori(dev) => {
                let id = alloc_id(next_id);
                let top = stack.last_mut().ok_or(MnemonicParseError::Syntax {
                    line: token.line,
                    message: "ORI without prior LD".to_string(),
                })?;
                top.elements.push(RungElement::Contact(Contact {
                    variable: dev.clone(),
                    contact_type: ContactType::NormallyClosed,
                    local_id: id,
                }));
                top.last_id = Some(id);
            }
            Instruction::Orb => {
                // Combine top two stack entries as parallel (OR)
                if stack.len() < 2 {
                    return Err(MnemonicParseError::Syntax {
                        line: token.line,
                        message: "ORB requires at least 2 items on stack".to_string(),
                    });
                }
                let b = stack.pop().unwrap();
                let a = stack.last_mut().unwrap();

                // Merge: both paths are parallel, keep elements from both.
                a.elements.extend(b.elements);
                a.connections.extend(b.connections);
                // The output of the combined node can come from either path.
                // Keep b's last_id as the new last_id (both are valid outputs).
                if let (Some(_a_last), Some(b_last)) = (a.last_id, b.last_id) {
                    // Create a virtual merge - downstream will connect to both
                    // We track both by keeping b_last as last_id
                    // The actual OR merge happens at the output coil
                    a.last_id = Some(b_last);
                    // Keep a_last as well by not overwriting connections
                }
            }
            Instruction::Anb => {
                // Combine top two stack entries as series (AND)
                if stack.len() < 2 {
                    return Err(MnemonicParseError::Syntax {
                        line: token.line,
                        message: "ANB requires at least 2 items on stack".to_string(),
                    });
                }
                let b = stack.pop().unwrap();
                let a = stack.last_mut().unwrap();

                // Series: connect a's output to b's first element
                if let Some(a_last) = a.last_id {
                    // Find b's first element id
                    if let Some(b_first) = b.elements.first().map(|e| match e {
                        RungElement::Contact(c) => c.local_id,
                        RungElement::Coil(c) => c.local_id,
                        RungElement::Block(bl) => bl.local_id,
                    }) {
                        a.connections.push(Connection {
                            from_id: a_last,
                            to_id: b_first,
                        });
                    }
                }

                a.elements.extend(b.elements);
                a.connections.extend(b.connections);
                a.last_id = b.last_id;
            }
            Instruction::Mps => {
                // Push current result point for branching
                if let Some(top) = stack.last() {
                    if let Some(id) = top.last_id {
                        branch_stack.push(id);
                    }
                }
            }
            Instruction::Mrd => {
                // Read (peek) the branch point - restore last_id
                if let Some(&branch_id) = branch_stack.last() {
                    if let Some(top) = stack.last_mut() {
                        top.last_id = Some(branch_id);
                    }
                }
            }
            Instruction::Mpp => {
                // Pop the branch point and restore
                if let Some(branch_id) = branch_stack.pop() {
                    if let Some(top) = stack.last_mut() {
                        top.last_id = Some(branch_id);
                    }
                }
            }
            Instruction::Out(dev) => {
                let id = alloc_id(next_id);
                emit_output(&mut stack, &mut all_elements, &mut all_connections, id,
                    RungElement::Coil(Coil {
                        variable: dev.clone(),
                        coil_type: CoilType::Normal,
                        local_id: id,
                    }),
                    token.line,
                )?;
            }
            Instruction::Set(dev) => {
                let id = alloc_id(next_id);
                emit_output(&mut stack, &mut all_elements, &mut all_connections, id,
                    RungElement::Coil(Coil {
                        variable: dev.clone(),
                        coil_type: CoilType::Set,
                        local_id: id,
                    }),
                    token.line,
                )?;
            }
            Instruction::Rst(dev) => {
                let id = alloc_id(next_id);
                emit_output(&mut stack, &mut all_elements, &mut all_connections, id,
                    RungElement::Coil(Coil {
                        variable: dev.clone(),
                        coil_type: CoilType::Reset,
                        local_id: id,
                    }),
                    token.line,
                )?;
            }
            Instruction::OutTimer(timer_dev, preset) => {
                let id = alloc_id(next_id);
                emit_output(&mut stack, &mut all_elements, &mut all_connections, id,
                    RungElement::Block(Block {
                        type_name: "TON".to_string(),
                        instance_name: timer_dev.clone(),
                        local_id: id,
                        parameters: vec![
                            ("IN".to_string(), String::new()),
                            ("PT".to_string(), preset.clone()),
                        ],
                    }),
                    token.line,
                )?;
            }
            Instruction::OutCounter(counter_dev, preset) => {
                let id = alloc_id(next_id);
                emit_output(&mut stack, &mut all_elements, &mut all_connections, id,
                    RungElement::Block(Block {
                        type_name: "CTU".to_string(),
                        instance_name: counter_dev.clone(),
                        local_id: id,
                        parameters: vec![
                            ("CU".to_string(), String::new()),
                            ("PV".to_string(), preset.clone()),
                        ],
                    }),
                    token.line,
                )?;
            }
            Instruction::End => break,
        }
    }

    // Flush remaining stack into elements
    for node in &stack {
        all_elements.extend(node.elements.iter().cloned());
        all_connections.extend(node.connections.iter().cloned());
    }

    Ok(Rung {
        comment: None,
        elements: all_elements,
        connections: all_connections,
    })
}

fn alloc_id(next_id: &mut u32) -> u32 {
    let id = *next_id;
    *next_id += 1;
    id
}

fn emit_output(
    stack: &mut [LogicNode],
    all_elements: &mut Vec<RungElement>,
    all_connections: &mut Vec<Connection>,
    output_id: u32,
    output_element: RungElement,
    line: usize,
) -> Result<(), MnemonicParseError> {
    let top = stack.last().ok_or(MnemonicParseError::Syntax {
        line,
        message: "output instruction without prior LD".to_string(),
    })?;

    // Find leaf elements (end of chains) to connect to the output.
    let all_elem_ids: std::collections::HashSet<u32> = top
        .elements
        .iter()
        .map(|e| match e {
            RungElement::Contact(c) => c.local_id,
            RungElement::Coil(c) => c.local_id,
            RungElement::Block(b) => b.local_id,
        })
        .collect();

    let connected_from: std::collections::HashSet<u32> = top
        .connections
        .iter()
        .map(|c| c.from_id)
        .collect();

    let leaf_ids: Vec<u32> = all_elem_ids
        .iter()
        .filter(|id| !connected_from.contains(id))
        .copied()
        .collect();

    if !leaf_ids.is_empty() {
        for &leaf_id in &leaf_ids {
            all_connections.push(Connection {
                from_id: leaf_id,
                to_id: output_id,
            });
        }
    } else if let Some(last_id) = top.last_id {
        all_connections.push(Connection {
            from_id: last_id,
            to_id: output_id,
        });
    }

    // Only add the output element itself; stack elements are flushed at the end.
    all_elements.push(output_element);

    Ok(())
}

// ── Writer ───────────────────────────────────────────────────────────

/// Write a Project to Mitsubishi mnemonic format.
pub fn write_mnemonic(project: &Project) -> String {
    let mut buf = String::new();

    for program in &project.programs {
        buf.push_str(&format!("; Program: {}\n", program.name));

        for (rung_idx, rung) in program.rungs.iter().enumerate() {
            buf.push_str(&format!("; Rung {}\n", rung_idx + 1));
            write_rung_mnemonic(rung, &mut buf);
        }
    }

    buf.push_str("END\n");
    buf
}

fn write_rung_mnemonic(rung: &Rung, buf: &mut String) {
    // Build connection graph
    let mut incoming: std::collections::HashMap<u32, Vec<u32>> = std::collections::HashMap::new();
    for conn in &rung.connections {
        incoming.entry(conn.to_id).or_default().push(conn.from_id);
    }

    let elem_ids: std::collections::HashSet<u32> = rung
        .elements
        .iter()
        .map(|e| match e {
            RungElement::Contact(c) => c.local_id,
            RungElement::Coil(c) => c.local_id,
            RungElement::Block(b) => b.local_id,
        })
        .collect();

    // Find root contacts (those with no incoming connection from another element)
    let connected_to: std::collections::HashSet<u32> = incoming
        .iter()
        .filter(|(_, froms)| froms.iter().any(|f| elem_ids.contains(f)))
        .map(|(&to_id, _)| to_id)
        .collect();

    // Find output elements (coils and blocks that are leaf nodes)
    let outputs: Vec<&RungElement> = rung
        .elements
        .iter()
        .filter(|e| matches!(e, RungElement::Coil(_) | RungElement::Block(_)))
        .collect();

    // Simple approach: emit contacts in order, then outputs
    let contacts: Vec<&Contact> = rung
        .elements
        .iter()
        .filter_map(|e| match e {
            RungElement::Contact(c) => Some(c),
            _ => None,
        })
        .collect();

    // Determine series chain order by following connections
    let root_contacts: Vec<&Contact> = contacts
        .iter()
        .filter(|c| !connected_to.contains(&c.local_id))
        .copied()
        .collect();

    if root_contacts.is_empty() && !contacts.is_empty() {
        // Fallback: just emit all contacts as LD/AND
        let mut first = true;
        for c in &contacts {
            let instr = if first {
                first = false;
                match c.contact_type {
                    ContactType::NormallyOpen => "LD",
                    ContactType::NormallyClosed => "LDI",
                }
            } else {
                match c.contact_type {
                    ContactType::NormallyOpen => "AND",
                    ContactType::NormallyClosed => "ANI",
                }
            };
            buf.push_str(&format!("{:<5}{}\n", instr, c.variable));
        }
    } else {
        // Follow the chain from each root
        let mut emitted: std::collections::HashSet<u32> = std::collections::HashSet::new();
        let mut first_root = true;

        for root in &root_contacts {
            if first_root {
                let instr = match root.contact_type {
                    ContactType::NormallyOpen => "LD",
                    ContactType::NormallyClosed => "LDI",
                };
                buf.push_str(&format!("{:<5}{}\n", instr, root.variable));
                first_root = false;
            } else {
                // Additional root = parallel path, use OR
                let instr = match root.contact_type {
                    ContactType::NormallyOpen => "OR",
                    ContactType::NormallyClosed => "ORI",
                };
                buf.push_str(&format!("{:<5}{}\n", instr, root.variable));
            }
            emitted.insert(root.local_id);

            // Follow series chain
            emit_chain(root.local_id, rung, &incoming, &mut emitted, buf);
        }
    }

    // Emit outputs
    for output in &outputs {
        match output {
            RungElement::Coil(c) => {
                let instr = match c.coil_type {
                    CoilType::Normal => "OUT",
                    CoilType::Set => "SET",
                    CoilType::Reset => "RST",
                };
                buf.push_str(&format!("{:<5}{}\n", instr, c.variable));
            }
            RungElement::Block(b) => {
                match b.type_name.as_str() {
                    "TON" | "TOF" | "TP" => {
                        let preset = b
                            .parameters
                            .iter()
                            .find(|(k, _)| k == "PT")
                            .map(|(_, v)| v.as_str())
                            .unwrap_or("K0");
                        buf.push_str(&format!("OUT  {} {}\n", b.instance_name, preset));
                    }
                    "CTU" | "CTD" => {
                        let preset = b
                            .parameters
                            .iter()
                            .find(|(k, _)| k == "PV")
                            .map(|(_, v)| v.as_str())
                            .unwrap_or("K0");
                        buf.push_str(&format!("OUT  {} {}\n", b.instance_name, preset));
                    }
                    _ => {
                        buf.push_str(&format!("; BLOCK {} {}\n", b.type_name, b.instance_name));
                    }
                }
            }
            _ => {}
        }
    }
}

fn emit_chain(
    from_id: u32,
    rung: &Rung,
    incoming: &std::collections::HashMap<u32, Vec<u32>>,
    emitted: &mut std::collections::HashSet<u32>,
    buf: &mut String,
) {
    // Find elements that have from_id as their source
    for elem in &rung.elements {
        let (elem_id, _is_contact) = match elem {
            RungElement::Contact(c) => (c.local_id, true),
            _ => continue,
        };

        if emitted.contains(&elem_id) {
            continue;
        }

        if let Some(sources) = incoming.get(&elem_id) {
            if sources.contains(&from_id) {
                emitted.insert(elem_id);
                if let RungElement::Contact(c) = elem {
                    let instr = match c.contact_type {
                        ContactType::NormallyOpen => "AND",
                        ContactType::NormallyClosed => "ANI",
                    };
                    buf.push_str(&format!("{:<5}{}\n", instr, c.variable));
                }
                // Continue chain
                emit_chain(elem_id, rung, incoming, emitted, buf);
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_simple_output() {
        let input = "LD X001\nOUT Y001\nEND\n";
        let project = parse_mnemonic(input, None, None).unwrap();

        assert_eq!(project.programs.len(), 1);
        let prog = &project.programs[0];
        assert_eq!(prog.rungs.len(), 1);

        let rung = &prog.rungs[0];
        assert_eq!(rung.elements.len(), 2);

        let contacts: Vec<_> = rung.elements.iter().filter_map(|e| match e {
            RungElement::Contact(c) => Some(c),
            _ => None,
        }).collect();
        assert_eq!(contacts.len(), 1);
        assert_eq!(contacts[0].variable, "X001");
        assert_eq!(contacts[0].contact_type, ContactType::NormallyOpen);

        let coils: Vec<_> = rung.elements.iter().filter_map(|e| match e {
            RungElement::Coil(c) => Some(c),
            _ => None,
        }).collect();
        assert_eq!(coils.len(), 1);
        assert_eq!(coils[0].variable, "Y001");
    }

    #[test]
    fn parse_self_hold() {
        let input = "\
LD  X001
AND X002
OR  Y001
OUT Y001
LD  X003
RST Y001
END
";
        let project = parse_mnemonic(input, Some("SelfHold"), None).unwrap();
        assert_eq!(project.name, "SelfHold");

        let prog = &project.programs[0];
        assert_eq!(prog.rungs.len(), 2);

        // Rung 1: X001, X002, Y001(contact), Y001(coil) = 4 elements
        let rung1 = &prog.rungs[0];
        let contacts: Vec<_> = rung1.elements.iter().filter_map(|e| match e {
            RungElement::Contact(c) => Some(c),
            _ => None,
        }).collect();
        assert_eq!(contacts.len(), 3); // X001, X002, Y001

        let coils: Vec<_> = rung1.elements.iter().filter_map(|e| match e {
            RungElement::Coil(c) => Some(c),
            _ => None,
        }).collect();
        assert_eq!(coils.len(), 1);
        assert_eq!(coils[0].variable, "Y001");
        assert_eq!(coils[0].coil_type, CoilType::Normal);

        // Rung 2: X003 -> RST Y001
        let rung2 = &prog.rungs[1];
        let coils2: Vec<_> = rung2.elements.iter().filter_map(|e| match e {
            RungElement::Coil(c) => Some(c),
            _ => None,
        }).collect();
        assert_eq!(coils2.len(), 1);
        assert_eq!(coils2[0].coil_type, CoilType::Reset);
    }

    #[test]
    fn parse_interlock() {
        let input = "\
LD  X001
ANI X002
OUT Y001
LD  X002
ANI X001
OUT Y002
END
";
        let project = parse_mnemonic(input, None, None).unwrap();
        let prog = &project.programs[0];
        assert_eq!(prog.rungs.len(), 2);

        // Rung 1: X001 NO, X002 NC -> Y001
        let r1_contacts: Vec<_> = prog.rungs[0].elements.iter().filter_map(|e| match e {
            RungElement::Contact(c) => Some(c),
            _ => None,
        }).collect();
        assert_eq!(r1_contacts[0].variable, "X001");
        assert_eq!(r1_contacts[0].contact_type, ContactType::NormallyOpen);
        assert_eq!(r1_contacts[1].variable, "X002");
        assert_eq!(r1_contacts[1].contact_type, ContactType::NormallyClosed);
    }

    #[test]
    fn parse_nc_contact() {
        let input = "LDI X010\nAND X001\nOUT Y001\nEND\n";
        let project = parse_mnemonic(input, None, None).unwrap();
        let rung = &project.programs[0].rungs[0];

        let contacts: Vec<_> = rung.elements.iter().filter_map(|e| match e {
            RungElement::Contact(c) => Some(c),
            _ => None,
        }).collect();
        assert_eq!(contacts[0].variable, "X010");
        assert_eq!(contacts[0].contact_type, ContactType::NormallyClosed);
        assert_eq!(contacts[1].variable, "X001");
        assert_eq!(contacts[1].contact_type, ContactType::NormallyOpen);
    }

    #[test]
    fn parse_timer() {
        let input = "LD  X001\nOUT T0 K100\nEND\n";
        let project = parse_mnemonic(input, None, None).unwrap();
        let rung = &project.programs[0].rungs[0];

        let blocks: Vec<_> = rung.elements.iter().filter_map(|e| match e {
            RungElement::Block(b) => Some(b),
            _ => None,
        }).collect();
        assert_eq!(blocks.len(), 1);
        assert_eq!(blocks[0].type_name, "TON");
        assert_eq!(blocks[0].instance_name, "T0");
        assert!(blocks[0].parameters.iter().any(|(k, v)| k == "PT" && v == "K100"));
    }

    #[test]
    fn parse_counter() {
        let input = "LD  X001\nOUT C0 K10\nEND\n";
        let project = parse_mnemonic(input, None, None).unwrap();
        let rung = &project.programs[0].rungs[0];

        let blocks: Vec<_> = rung.elements.iter().filter_map(|e| match e {
            RungElement::Block(b) => Some(b),
            _ => None,
        }).collect();
        assert_eq!(blocks.len(), 1);
        assert_eq!(blocks[0].type_name, "CTU");
        assert_eq!(blocks[0].instance_name, "C0");
        assert!(blocks[0].parameters.iter().any(|(k, v)| k == "PV" && v == "K10"));
    }

    #[test]
    fn parse_set_reset() {
        let input = "LD X001\nSET Y001\nLD X002\nRST Y001\nEND\n";
        let project = parse_mnemonic(input, None, None).unwrap();
        let prog = &project.programs[0];
        assert_eq!(prog.rungs.len(), 2);

        let coil1: Vec<_> = prog.rungs[0].elements.iter().filter_map(|e| match e {
            RungElement::Coil(c) => Some(c),
            _ => None,
        }).collect();
        assert_eq!(coil1[0].coil_type, CoilType::Set);

        let coil2: Vec<_> = prog.rungs[1].elements.iter().filter_map(|e| match e {
            RungElement::Coil(c) => Some(c),
            _ => None,
        }).collect();
        assert_eq!(coil2[0].coil_type, CoilType::Reset);
    }

    #[test]
    fn parse_comments_and_blank_lines() {
        let input = "\
; This is a comment
LD X001   ; inline comment

AND X002
OUT Y001
END
";
        let project = parse_mnemonic(input, None, None).unwrap();
        assert_eq!(project.programs[0].rungs[0].elements.len(), 3);
    }

    #[test]
    fn parse_orb_parallel() {
        // ORB pattern: two parallel branches
        // Branch 1: X001 AND X002
        // Branch 2: X003 AND X004
        // Combined with ORB, output to Y001
        let input = "\
LD  X001
AND X002
LD  X003
AND X004
ORB
OUT Y001
END
";
        let project = parse_mnemonic(input, None, None).unwrap();
        let rung = &project.programs[0].rungs[0];

        let contacts: Vec<_> = rung.elements.iter().filter_map(|e| match e {
            RungElement::Contact(c) => Some(c),
            _ => None,
        }).collect();
        assert_eq!(contacts.len(), 4);
        assert_eq!(contacts[0].variable, "X001");
        assert_eq!(contacts[1].variable, "X002");
        assert_eq!(contacts[2].variable, "X003");
        assert_eq!(contacts[3].variable, "X004");
    }

    #[test]
    fn parse_anb_series() {
        // ANB pattern: two blocks in series
        // Block 1: X001 OR X002
        // Block 2: X003 OR X004
        // Combined with ANB
        let input = "\
LD  X001
OR  X002
LD  X003
OR  X004
ANB
OUT Y001
END
";
        let project = parse_mnemonic(input, None, None).unwrap();
        let rung = &project.programs[0].rungs[0];

        let contacts: Vec<_> = rung.elements.iter().filter_map(|e| match e {
            RungElement::Contact(c) => Some(c),
            _ => None,
        }).collect();
        assert_eq!(contacts.len(), 4);
    }

    #[test]
    fn write_mnemonic_roundtrip() {
        let input = "\
LD  X001
AND X002
OUT Y001
LD  X003
RST Y001
END
";
        let project = parse_mnemonic(input, None, None).unwrap();
        let output = write_mnemonic(&project);

        assert!(output.contains("LD"));
        assert!(output.contains("X001"));
        assert!(output.contains("Y001"));
        assert!(output.contains("RST"));
        assert!(output.contains("END"));
    }

    #[test]
    fn error_on_empty_program() {
        let result = parse_mnemonic("", None, None);
        assert!(result.is_err());
    }

    #[test]
    fn error_on_missing_operand() {
        let result = parse_mnemonic("LD\nEND\n", None, None);
        assert!(result.is_err());
    }

    #[test]
    fn error_on_unknown_instruction() {
        let result = parse_mnemonic("FOOBAR X001\nEND\n", None, None);
        assert!(result.is_err());
    }
}

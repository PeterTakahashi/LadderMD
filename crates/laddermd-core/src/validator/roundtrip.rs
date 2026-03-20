use crate::model::*;
use crate::parser;
use crate::writer;

/// Result of roundtrip validation.
#[derive(Debug)]
pub struct ValidationResult {
    pub parse_ok: bool,
    pub roundtrip_ok: bool,
    pub total_rungs: usize,
    pub contacts: u32,
    pub coils: u32,
    pub blocks: u32,
    pub errors: Vec<String>,
}

/// Errors that can occur during validation.
#[derive(Debug, thiserror::Error)]
pub enum ValidateError {
    #[error("parse error: {0}")]
    Parse(#[from] parser::ParseError),
    #[error("write error: {0}")]
    Write(#[from] writer::WriteError),
}

/// Perform roundtrip validation: XML -> Model A -> XML -> Model B, then compare A and B.
pub fn validate(xml: &str) -> Result<ValidationResult, ValidateError> {
    // Step 1: Parse original XML to Model A
    let model_a = parser::parse(xml)?;

    // Count devices
    let mut contacts = 0u32;
    let mut coils = 0u32;
    let mut blocks = 0u32;
    let mut total_rungs = 0usize;

    for prog in &model_a.programs {
        total_rungs += prog.rungs.len();
        for rung in &prog.rungs {
            for elem in &rung.elements {
                match elem {
                    RungElement::Contact(_) => contacts += 1,
                    RungElement::Coil(_) => coils += 1,
                    RungElement::Block(_) => blocks += 1,
                }
            }
        }
    }

    // Step 2: Write Model A to XML
    let regenerated_xml = writer::write(&model_a)?;

    // Step 3: Parse regenerated XML to Model B
    let model_b = match parser::parse(&regenerated_xml) {
        Ok(m) => m,
        Err(e) => {
            return Ok(ValidationResult {
                parse_ok: true,
                roundtrip_ok: false,
                total_rungs,
                contacts,
                coils,
                blocks,
                errors: vec![format!("Failed to parse regenerated XML: {e}")],
            });
        }
    };

    // Step 4: Compare Model A and Model B (logical equivalence)
    let mut errors = Vec::new();
    compare_projects(&model_a, &model_b, &mut errors);

    Ok(ValidationResult {
        parse_ok: true,
        roundtrip_ok: errors.is_empty(),
        total_rungs,
        contacts,
        coils,
        blocks,
        errors,
    })
}

fn compare_projects(a: &Project, b: &Project, errors: &mut Vec<String>) {
    if a.name != b.name {
        errors.push(format!(
            "Project name mismatch: '{}' vs '{}'",
            a.name, b.name
        ));
    }

    if a.programs.len() != b.programs.len() {
        errors.push(format!(
            "Program count mismatch: {} vs {}",
            a.programs.len(),
            b.programs.len()
        ));
        return;
    }

    for (i, (pa, pb)) in a.programs.iter().zip(b.programs.iter()).enumerate() {
        if pa.name != pb.name {
            errors.push(format!(
                "Program[{}] name mismatch: '{}' vs '{}'",
                i, pa.name, pb.name
            ));
        }

        if pa.rungs.len() != pb.rungs.len() {
            errors.push(format!(
                "Program[{}] rung count mismatch: {} vs {}",
                i,
                pa.rungs.len(),
                pb.rungs.len()
            ));
            continue;
        }

        for (j, (ra, rb)) in pa.rungs.iter().zip(pb.rungs.iter()).enumerate() {
            compare_rungs(i, j, ra, rb, errors);
        }
    }
}

fn compare_rungs(prog_idx: usize, rung_idx: usize, a: &Rung, b: &Rung, errors: &mut Vec<String>) {
    let prefix = format!("Program[{}].Rung[{}]", prog_idx, rung_idx);

    if a.elements.len() != b.elements.len() {
        errors.push(format!(
            "{}: element count mismatch: {} vs {}",
            prefix,
            a.elements.len(),
            b.elements.len()
        ));
        return;
    }

    // Compare elements by type and variable (ignore localId differences, compare semantics)
    let a_contacts = extract_contacts(&a.elements);
    let b_contacts = extract_contacts(&b.elements);
    let a_coils = extract_coils(&a.elements);
    let b_coils = extract_coils(&b.elements);
    let a_blocks = extract_blocks(&a.elements);
    let b_blocks = extract_blocks(&b.elements);

    if a_contacts.len() != b_contacts.len() {
        errors.push(format!(
            "{}: contact count mismatch: {} vs {}",
            prefix,
            a_contacts.len(),
            b_contacts.len()
        ));
    } else {
        for (i, (ca, cb)) in a_contacts.iter().zip(b_contacts.iter()).enumerate() {
            if ca.variable != cb.variable {
                errors.push(format!(
                    "{}: contact[{}] variable mismatch: '{}' vs '{}'",
                    prefix, i, ca.variable, cb.variable
                ));
            }
            if ca.contact_type != cb.contact_type {
                errors.push(format!(
                    "{}: contact[{}] type mismatch: {:?} vs {:?}",
                    prefix, i, ca.contact_type, cb.contact_type
                ));
            }
        }
    }

    if a_coils.len() != b_coils.len() {
        errors.push(format!(
            "{}: coil count mismatch: {} vs {}",
            prefix,
            a_coils.len(),
            b_coils.len()
        ));
    } else {
        for (i, (ca, cb)) in a_coils.iter().zip(b_coils.iter()).enumerate() {
            if ca.variable != cb.variable {
                errors.push(format!(
                    "{}: coil[{}] variable mismatch: '{}' vs '{}'",
                    prefix, i, ca.variable, cb.variable
                ));
            }
            if ca.coil_type != cb.coil_type {
                errors.push(format!(
                    "{}: coil[{}] type mismatch: {:?} vs {:?}",
                    prefix, i, ca.coil_type, cb.coil_type
                ));
            }
        }
    }

    if a_blocks.len() != b_blocks.len() {
        errors.push(format!(
            "{}: block count mismatch: {} vs {}",
            prefix,
            a_blocks.len(),
            b_blocks.len()
        ));
    } else {
        for (i, (ba, bb)) in a_blocks.iter().zip(b_blocks.iter()).enumerate() {
            if ba.type_name != bb.type_name {
                errors.push(format!(
                    "{}: block[{}] typeName mismatch: '{}' vs '{}'",
                    prefix, i, ba.type_name, bb.type_name
                ));
            }
            if ba.instance_name != bb.instance_name {
                errors.push(format!(
                    "{}: block[{}] instanceName mismatch: '{}' vs '{}'",
                    prefix, i, ba.instance_name, bb.instance_name
                ));
            }
        }
    }

    // Compare connection topology (logical equivalence, ignoring exact localIds)
    // We compare by mapping: for each element, what elements connect to it?
    // We identify elements by (type, variable/name) instead of localId.
    compare_connection_topology(&prefix, a, b, errors);
}

fn extract_contacts(elements: &[RungElement]) -> Vec<&Contact> {
    elements
        .iter()
        .filter_map(|e| match e {
            RungElement::Contact(c) => Some(c),
            _ => None,
        })
        .collect()
}

fn extract_coils(elements: &[RungElement]) -> Vec<&Coil> {
    elements
        .iter()
        .filter_map(|e| match e {
            RungElement::Coil(c) => Some(c),
            _ => None,
        })
        .collect()
}

fn extract_blocks(elements: &[RungElement]) -> Vec<&Block> {
    elements
        .iter()
        .filter_map(|e| match e {
            RungElement::Block(b) => Some(b),
            _ => None,
        })
        .collect()
}

/// Compare connection topology using element identity keys instead of localIds.
fn compare_connection_topology(
    prefix: &str,
    a: &Rung,
    b: &Rung,
    errors: &mut Vec<String>,
) {
    // Create a canonical key for each element: "type:variable" or "block:typename:instance"
    fn element_key(elem: &RungElement) -> String {
        match elem {
            RungElement::Contact(c) => {
                let t = match c.contact_type {
                    ContactType::NormallyOpen => "NO",
                    ContactType::NormallyClosed => "NC",
                };
                format!("contact:{}:{}", c.variable, t)
            }
            RungElement::Coil(c) => {
                let t = match c.coil_type {
                    CoilType::Normal => "N",
                    CoilType::Set => "S",
                    CoilType::Reset => "R",
                };
                format!("coil:{}:{}", c.variable, t)
            }
            RungElement::Block(b) => {
                format!("block:{}:{}", b.type_name, b.instance_name)
            }
        }
    }

    fn element_id(elem: &RungElement) -> u32 {
        match elem {
            RungElement::Contact(c) => c.local_id,
            RungElement::Coil(c) => c.local_id,
            RungElement::Block(b) => b.local_id,
        }
    }

    // Build id->key maps
    let a_id_to_key: std::collections::HashMap<u32, String> = a
        .elements
        .iter()
        .map(|e| (element_id(e), element_key(e)))
        .collect();

    let b_id_to_key: std::collections::HashMap<u32, String> = b
        .elements
        .iter()
        .map(|e| (element_id(e), element_key(e)))
        .collect();

    // Convert connections from id-based to key-based
    let a_conn_keys: std::collections::HashSet<(String, String)> = a
        .connections
        .iter()
        .filter_map(|c| {
            let from = a_id_to_key.get(&c.from_id).cloned().unwrap_or_default();
            let to = a_id_to_key.get(&c.to_id).cloned().unwrap_or_default();
            if from.is_empty() || to.is_empty() {
                None // connection to/from power rail, skip
            } else {
                Some((from, to))
            }
        })
        .collect();

    let b_conn_keys: std::collections::HashSet<(String, String)> = b
        .connections
        .iter()
        .filter_map(|c| {
            let from = b_id_to_key.get(&c.from_id).cloned().unwrap_or_default();
            let to = b_id_to_key.get(&c.to_id).cloned().unwrap_or_default();
            if from.is_empty() || to.is_empty() {
                None
            } else {
                Some((from, to))
            }
        })
        .collect();

    if a_conn_keys != b_conn_keys {
        let missing = a_conn_keys.difference(&b_conn_keys).collect::<Vec<_>>();
        let extra = b_conn_keys.difference(&a_conn_keys).collect::<Vec<_>>();
        if !missing.is_empty() {
            errors.push(format!(
                "{}: missing connections in roundtrip: {:?}",
                prefix, missing
            ));
        }
        if !extra.is_empty() {
            errors.push(format!(
                "{}: extra connections in roundtrip: {:?}",
                prefix, extra
            ));
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn fixture(name: &str) -> String {
        let path = format!("../../tests/fixtures/{name}");
        std::fs::read_to_string(&path).unwrap()
    }

    #[test]
    fn roundtrip_self_hold() {
        let result = validate(&fixture("self_hold.xml")).unwrap();
        assert!(result.parse_ok);
        assert!(
            result.roundtrip_ok,
            "Roundtrip failed: {:?}",
            result.errors
        );
        assert_eq!(result.total_rungs, 2);
        assert_eq!(result.contacts, 4);
        assert_eq!(result.coils, 2);
    }

    #[test]
    fn roundtrip_interlock() {
        let result = validate(&fixture("interlock.xml")).unwrap();
        assert!(result.parse_ok);
        assert!(
            result.roundtrip_ok,
            "Roundtrip failed: {:?}",
            result.errors
        );
        assert_eq!(result.total_rungs, 2);
        assert_eq!(result.contacts, 4);
        assert_eq!(result.coils, 2);
    }

    #[test]
    fn roundtrip_timer() {
        let result = validate(&fixture("timer.xml")).unwrap();
        assert!(result.parse_ok);
        assert!(
            result.roundtrip_ok,
            "Roundtrip failed: {:?}",
            result.errors
        );
        assert_eq!(result.total_rungs, 2);
        assert_eq!(result.blocks, 1);
    }

    #[test]
    fn roundtrip_emergency_stop() {
        let result = validate(&fixture("emergency_stop.xml")).unwrap();
        assert!(result.parse_ok);
        assert!(
            result.roundtrip_ok,
            "Roundtrip failed: {:?}",
            result.errors
        );
        assert_eq!(result.total_rungs, 1);
        assert_eq!(result.contacts, 3);
        assert_eq!(result.coils, 1);
    }
}

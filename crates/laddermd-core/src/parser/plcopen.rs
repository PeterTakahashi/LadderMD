use quick_xml::de::from_str;
use serde::Deserialize;
use std::collections::HashMap;

use crate::model;

/// Errors that can occur during PLCopen XML parsing.
#[derive(Debug, thiserror::Error)]
pub enum ParseError {
    #[error("XML deserialization error: {0}")]
    Xml(#[from] quick_xml::DeError),

    #[error("invalid element at localId={local_id}: {reason}")]
    InvalidElement { local_id: u32, reason: String },

    #[error("no LD body found in POU '{pou_name}'")]
    NoLdBody { pou_name: String },
}

// ── PLCopen XML serde types ──────────────────────────────────────────

#[derive(Debug, Deserialize)]
#[serde(rename = "project")]
struct XmlProject {
    #[serde(rename = "contentHeader")]
    content_header: XmlContentHeader,
    types: XmlTypes,
}

#[derive(Debug, Deserialize)]
struct XmlContentHeader {
    #[serde(rename = "@name")]
    name: String,
}

#[derive(Debug, Deserialize)]
struct XmlTypes {
    pous: XmlPous,
}

#[derive(Debug, Deserialize)]
struct XmlPous {
    #[serde(rename = "pou", default)]
    pous: Vec<XmlPou>,
}

#[derive(Debug, Deserialize)]
struct XmlPou {
    #[serde(rename = "@name")]
    name: String,
    body: XmlBody,
}

#[derive(Debug, Deserialize)]
struct XmlBody {
    #[serde(rename = "LD")]
    ld: Option<XmlLd>,
}

#[derive(Debug, Deserialize)]
struct XmlLd {
    #[serde(rename = "$value", default)]
    elements: Vec<XmlLdElement>,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
enum XmlLdElement {
    LeftPowerRail(XmlLeftPowerRail),
    RightPowerRail(XmlRightPowerRail),
    Contact(XmlContact),
    Coil(XmlCoil),
    Block(XmlBlock),
    Comment(XmlComment),
}

#[derive(Debug, Deserialize)]
struct XmlLeftPowerRail {
    #[serde(rename = "@localId")]
    local_id: u32,
    position: XmlPosition,
    #[serde(rename = "connectionPointOut", default)]
    connection_point_out: Vec<XmlConnectionPointOut>,
}

#[derive(Debug, Deserialize)]
struct XmlRightPowerRail {
    #[serde(rename = "@localId")]
    local_id: u32,
}

#[derive(Debug, Deserialize)]
struct XmlContact {
    #[serde(rename = "@localId")]
    local_id: u32,
    #[serde(rename = "@negated", default)]
    negated: bool,
    position: XmlPosition,
    #[serde(rename = "connectionPointIn")]
    connection_point_in: Option<XmlConnectionPointIn>,
    variable: String,
}

#[derive(Debug, Deserialize)]
struct XmlCoil {
    #[serde(rename = "@localId")]
    local_id: u32,
    #[serde(rename = "@negated", default)]
    negated: bool,
    #[serde(rename = "@storage")]
    storage: Option<String>,
    position: XmlPosition,
    #[serde(rename = "connectionPointIn")]
    connection_point_in: Option<XmlConnectionPointIn>,
    variable: String,
}

#[derive(Debug, Deserialize)]
struct XmlBlock {
    #[serde(rename = "@localId")]
    local_id: u32,
    #[serde(rename = "@typeName")]
    type_name: String,
    #[serde(rename = "@instanceName")]
    instance_name: Option<String>,
    position: XmlPosition,
    #[serde(rename = "inputVariables")]
    input_variables: Option<XmlInputVariables>,
    #[serde(rename = "outputVariables")]
    output_variables: Option<XmlOutputVariables>,
}

#[derive(Debug, Deserialize)]
struct XmlInputVariables {
    #[serde(rename = "variable", default)]
    variables: Vec<XmlBlockVariable>,
}

#[derive(Debug, Deserialize)]
struct XmlOutputVariables {
    #[serde(rename = "variable", default)]
    variables: Vec<XmlBlockVariable>,
}

#[derive(Debug, Deserialize)]
struct XmlBlockVariable {
    #[serde(rename = "@formalParameter")]
    formal_parameter: String,
    #[serde(rename = "connectionPointIn")]
    connection_point_in: Option<XmlConnectionPointIn>,
}

#[derive(Debug, Deserialize)]
struct XmlComment {
    #[serde(rename = "@localId")]
    local_id: Option<u32>,
    content: Option<String>,
}

#[derive(Debug, Deserialize)]
struct XmlPosition {
    #[serde(rename = "@x")]
    x: i32,
    #[serde(rename = "@y")]
    y: i32,
}

#[derive(Debug, Deserialize)]
struct XmlConnectionPointIn {
    #[serde(rename = "connection", default)]
    connections: Vec<XmlConnection>,
}

#[derive(Debug, Deserialize)]
struct XmlConnectionPointOut {}

#[derive(Debug, Deserialize)]
struct XmlConnection {
    #[serde(rename = "@refLocalId")]
    ref_local_id: u32,
}

// ── Public API ───────────────────────────────────────────────────────

/// Parse a PLCopen XML string into our internal model.
pub fn parse(xml: &str) -> Result<model::Project, ParseError> {
    // Strip the namespace to simplify serde deserialization.
    // quick-xml's serde support has limited namespace handling.
    let xml_no_ns = xml.replace("xmlns=\"http://www.plcopen.org/xml/tc6_0201\"", "");

    let xml_project: XmlProject = from_str(&xml_no_ns)?;

    let mut programs = Vec::new();
    for pou in &xml_project.types.pous.pous {
        let ld = pou
            .body
            .ld
            .as_ref()
            .ok_or_else(|| ParseError::NoLdBody {
                pou_name: pou.name.clone(),
            })?;

        let rungs = build_rungs(&ld.elements)?;
        programs.push(model::Program {
            name: pou.name.clone(),
            rungs,
        });
    }

    Ok(model::Project {
        name: xml_project.content_header.name.clone(),
        programs,
    })
}

/// Build rungs from the flat list of LD elements.
///
/// Each `leftPowerRail` starts a new rung. All contacts, coils, and blocks
/// that are reachable from that power rail (via refLocalId connections)
/// belong to the same rung.
fn build_rungs(elements: &[XmlLdElement]) -> Result<Vec<model::Rung>, ParseError> {
    // Step 1: Collect all elements by localId, and record power rail positions.
    let mut power_rail_ids: Vec<(u32, i32)> = Vec::new(); // (localId, y-position)
    let mut element_positions: HashMap<u32, i32> = HashMap::new(); // localId -> y

    for elem in elements {
        match elem {
            XmlLdElement::LeftPowerRail(rail) => {
                power_rail_ids.push((rail.local_id, rail.position.y));
                element_positions.insert(rail.local_id, rail.position.y);
            }
            XmlLdElement::Contact(c) => {
                element_positions.insert(c.local_id, c.position.y);
            }
            XmlLdElement::Coil(c) => {
                element_positions.insert(c.local_id, c.position.y);
            }
            XmlLdElement::Block(b) => {
                element_positions.insert(b.local_id, b.position.y);
            }
            _ => {}
        }
    }

    // Sort power rails by y-position to determine rung order.
    power_rail_ids.sort_by_key(|&(_, y)| y);

    // Step 2: For each element, find which power rail it belongs to by tracing
    // connections back to a leftPowerRail.
    let rail_id_set: std::collections::HashSet<u32> =
        power_rail_ids.iter().map(|&(id, _)| id).collect();

    // Build a map of localId -> incoming connections (refLocalIds)
    let mut incoming: HashMap<u32, Vec<u32>> = HashMap::new();
    for elem in elements {
        match elem {
            XmlLdElement::Contact(c) => {
                if let Some(ref cpi) = c.connection_point_in {
                    let refs: Vec<u32> = cpi.connections.iter().map(|c| c.ref_local_id).collect();
                    incoming.insert(c.local_id, refs);
                }
            }
            XmlLdElement::Coil(c) => {
                if let Some(ref cpi) = c.connection_point_in {
                    let refs: Vec<u32> = cpi.connections.iter().map(|c| c.ref_local_id).collect();
                    incoming.insert(c.local_id, refs);
                }
            }
            XmlLdElement::Block(b) => {
                if let Some(ref iv) = b.input_variables {
                    let mut refs = Vec::new();
                    for var in &iv.variables {
                        if let Some(ref cpi) = var.connection_point_in {
                            for conn in &cpi.connections {
                                refs.push(conn.ref_local_id);
                            }
                        }
                    }
                    if !refs.is_empty() {
                        incoming.insert(b.local_id, refs);
                    }
                }
            }
            _ => {}
        }
    }

    // Trace back from each element to find its root power rail.
    fn find_root_rail(
        local_id: u32,
        incoming: &HashMap<u32, Vec<u32>>,
        rail_ids: &std::collections::HashSet<u32>,
        visited: &mut std::collections::HashSet<u32>,
    ) -> Option<u32> {
        if rail_ids.contains(&local_id) {
            return Some(local_id);
        }
        if !visited.insert(local_id) {
            return None; // cycle prevention
        }
        if let Some(refs) = incoming.get(&local_id) {
            for &ref_id in refs {
                if let Some(rail) = find_root_rail(ref_id, incoming, rail_ids, visited) {
                    return Some(rail);
                }
            }
        }
        None
    }

    // Step 3: Group elements by their root power rail.
    let mut rung_elements: HashMap<u32, Vec<&XmlLdElement>> = HashMap::new();
    for &(rail_id, _) in &power_rail_ids {
        rung_elements.entry(rail_id).or_default();
    }

    for elem in elements {
        let local_id = match elem {
            XmlLdElement::Contact(c) => c.local_id,
            XmlLdElement::Coil(c) => c.local_id,
            XmlLdElement::Block(b) => b.local_id,
            _ => continue,
        };

        let mut visited = std::collections::HashSet::new();
        if let Some(rail_id) = find_root_rail(local_id, &incoming, &rail_id_set, &mut visited) {
            rung_elements.entry(rail_id).or_default().push(elem);
        }
    }

    // Step 4: Convert grouped elements into Rungs.
    let mut rungs = Vec::new();
    for &(rail_id, _) in &power_rail_ids {
        let elems = rung_elements.get(&rail_id).map(|v| v.as_slice()).unwrap_or(&[]);

        let mut rung_model_elements = Vec::new();
        let mut connections = Vec::new();

        for elem in elems {
            match elem {
                XmlLdElement::Contact(c) => {
                    let contact_type = if c.negated {
                        model::ContactType::NormallyClosed
                    } else {
                        model::ContactType::NormallyOpen
                    };
                    rung_model_elements.push(model::RungElement::Contact(model::Contact {
                        variable: c.variable.clone(),
                        contact_type,
                        local_id: c.local_id,
                    }));

                    if let Some(ref cpi) = c.connection_point_in {
                        for conn in &cpi.connections {
                            connections.push(model::Connection {
                                from_id: conn.ref_local_id,
                                to_id: c.local_id,
                            });
                        }
                    }
                }
                XmlLdElement::Coil(c) => {
                    let coil_type = match c.storage.as_deref() {
                        Some("set") => model::CoilType::Set,
                        Some("reset") => model::CoilType::Reset,
                        _ => model::CoilType::Normal,
                    };
                    rung_model_elements.push(model::RungElement::Coil(model::Coil {
                        variable: c.variable.clone(),
                        coil_type,
                        local_id: c.local_id,
                    }));

                    if let Some(ref cpi) = c.connection_point_in {
                        for conn in &cpi.connections {
                            connections.push(model::Connection {
                                from_id: conn.ref_local_id,
                                to_id: c.local_id,
                            });
                        }
                    }
                }
                XmlLdElement::Block(b) => {
                    let mut parameters = Vec::new();
                    if let Some(ref iv) = b.input_variables {
                        for var in &iv.variables {
                            parameters.push((var.formal_parameter.clone(), String::new()));
                        }
                    }
                    rung_model_elements.push(model::RungElement::Block(model::Block {
                        type_name: b.type_name.clone(),
                        instance_name: b.instance_name.clone().unwrap_or_default(),
                        local_id: b.local_id,
                        parameters,
                    }));

                    if let Some(ref iv) = b.input_variables {
                        for var in &iv.variables {
                            if let Some(ref cpi) = var.connection_point_in {
                                for conn in &cpi.connections {
                                    connections.push(model::Connection {
                                        from_id: conn.ref_local_id,
                                        to_id: b.local_id,
                                    });
                                }
                            }
                        }
                    }
                }
                _ => {}
            }
        }

        rungs.push(model::Rung {
            comment: None,
            elements: rung_model_elements,
            connections,
        });
    }

    Ok(rungs)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn fixture(name: &str) -> String {
        let path = format!("../../tests/fixtures/{name}");
        std::fs::read_to_string(&path).unwrap_or_else(|e| panic!("Failed to read {path}: {e}"))
    }

    #[test]
    fn parse_self_hold() {
        let xml = fixture("self_hold.xml");
        let project = parse(&xml).unwrap();

        assert_eq!(project.name, "SelfHoldTest");
        assert_eq!(project.programs.len(), 1);

        let prog = &project.programs[0];
        assert_eq!(prog.name, "Main");
        assert_eq!(prog.rungs.len(), 2);

        // Rung 1: X001, X002, Y001(contact), Y001(coil)
        let rung1 = &prog.rungs[0];
        assert_eq!(rung1.elements.len(), 4);

        let contacts: Vec<_> = rung1
            .elements
            .iter()
            .filter_map(|e| match e {
                model::RungElement::Contact(c) => Some(c),
                _ => None,
            })
            .collect();
        assert_eq!(contacts.len(), 3);
        assert_eq!(contacts[0].variable, "X001");
        assert_eq!(contacts[0].contact_type, model::ContactType::NormallyOpen);
        assert_eq!(contacts[1].variable, "X002");
        assert_eq!(contacts[2].variable, "Y001");

        let coils: Vec<_> = rung1
            .elements
            .iter()
            .filter_map(|e| match e {
                model::RungElement::Coil(c) => Some(c),
                _ => None,
            })
            .collect();
        assert_eq!(coils.len(), 1);
        assert_eq!(coils[0].variable, "Y001");
        assert_eq!(coils[0].coil_type, model::CoilType::Normal);

        // Rung 2: X003 contact, Y001 reset coil
        let rung2 = &prog.rungs[1];
        assert_eq!(rung2.elements.len(), 2);

        let coils2: Vec<_> = rung2
            .elements
            .iter()
            .filter_map(|e| match e {
                model::RungElement::Coil(c) => Some(c),
                _ => None,
            })
            .collect();
        assert_eq!(coils2.len(), 1);
        assert_eq!(coils2[0].coil_type, model::CoilType::Reset);
    }

    #[test]
    fn parse_interlock() {
        let xml = fixture("interlock.xml");
        let project = parse(&xml).unwrap();

        assert_eq!(project.name, "InterlockTest");
        let prog = &project.programs[0];
        assert_eq!(prog.rungs.len(), 2);

        // Rung 1: X001(NO), X002(NC) -> Y001
        let rung1 = &prog.rungs[0];
        let contacts: Vec<_> = rung1
            .elements
            .iter()
            .filter_map(|e| match e {
                model::RungElement::Contact(c) => Some(c),
                _ => None,
            })
            .collect();
        assert_eq!(contacts.len(), 2);
        assert_eq!(contacts[0].variable, "X001");
        assert_eq!(contacts[0].contact_type, model::ContactType::NormallyOpen);
        assert_eq!(contacts[1].variable, "X002");
        assert_eq!(
            contacts[1].contact_type,
            model::ContactType::NormallyClosed
        );

        // Rung 2: X002(NO), X001(NC) -> Y002
        let rung2 = &prog.rungs[1];
        let contacts2: Vec<_> = rung2
            .elements
            .iter()
            .filter_map(|e| match e {
                model::RungElement::Contact(c) => Some(c),
                _ => None,
            })
            .collect();
        assert_eq!(contacts2[0].variable, "X002");
        assert_eq!(contacts2[0].contact_type, model::ContactType::NormallyOpen);
        assert_eq!(contacts2[1].variable, "X001");
        assert_eq!(
            contacts2[1].contact_type,
            model::ContactType::NormallyClosed
        );
    }

    #[test]
    fn parse_timer() {
        let xml = fixture("timer.xml");
        let project = parse(&xml).unwrap();

        assert_eq!(project.name, "TimerTest");
        let prog = &project.programs[0];
        assert_eq!(prog.rungs.len(), 2);

        // Rung 1: X001 contact + TON block
        let rung1 = &prog.rungs[0];
        let blocks: Vec<_> = rung1
            .elements
            .iter()
            .filter_map(|e| match e {
                model::RungElement::Block(b) => Some(b),
                _ => None,
            })
            .collect();
        assert_eq!(blocks.len(), 1);
        assert_eq!(blocks[0].type_name, "TON");
        assert_eq!(blocks[0].instance_name, "T001_Instance");

        // Rung 2: T001 contact -> Y001 coil
        let rung2 = &prog.rungs[1];
        assert_eq!(rung2.elements.len(), 2);
    }

    #[test]
    fn parse_emergency_stop() {
        let xml = fixture("emergency_stop.xml");
        let project = parse(&xml).unwrap();

        assert_eq!(project.name, "EmergencyStopTest");
        let prog = &project.programs[0];
        assert_eq!(prog.rungs.len(), 1);

        let rung = &prog.rungs[0];
        // X010(NC), X001(NO), Y001(NO) contacts + Y001 coil = 4 elements
        assert_eq!(rung.elements.len(), 4);

        let contacts: Vec<_> = rung
            .elements
            .iter()
            .filter_map(|e| match e {
                model::RungElement::Contact(c) => Some(c),
                _ => None,
            })
            .collect();
        assert_eq!(contacts[0].variable, "X010");
        assert_eq!(
            contacts[0].contact_type,
            model::ContactType::NormallyClosed
        );
    }

    #[test]
    fn connections_are_tracked() {
        let xml = fixture("self_hold.xml");
        let project = parse(&xml).unwrap();
        let rung1 = &project.programs[0].rungs[0];

        // Y001 coil (localId=5) should have connections from X002(3) and Y001 contact(4)
        let coil_connections: Vec<_> = rung1
            .connections
            .iter()
            .filter(|c| c.to_id == 5)
            .collect();
        assert_eq!(coil_connections.len(), 2);
        assert!(coil_connections.iter().any(|c| c.from_id == 3)); // from X002
        assert!(coil_connections.iter().any(|c| c.from_id == 4)); // from Y001 contact
    }
}

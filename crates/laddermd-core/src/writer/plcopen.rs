use std::fmt::Write;

use crate::model::*;

/// Errors that can occur during PLCopen XML writing.
#[derive(Debug, thiserror::Error)]
pub enum WriteError {
    #[error("format error: {0}")]
    Format(#[from] std::fmt::Error),
}

/// Write a Project to PLCopen XML format.
pub fn write(project: &Project) -> Result<String, WriteError> {
    let mut buf = String::new();
    writeln!(buf, "<?xml version=\"1.0\" encoding=\"UTF-8\"?>")?;
    writeln!(
        buf,
        "<project xmlns=\"http://www.plcopen.org/xml/tc6_0201\">"
    )?;
    writeln!(buf, "  <fileHeader companyName=\"\" productName=\"\" productVersion=\"\" creationDateTime=\"2024-01-01T00:00:00\"/>")?;
    writeln!(buf, "  <contentHeader name=\"{}\" modificationDateTime=\"2024-01-01T00:00:00\">", xml_escape(&project.name))?;
    writeln!(buf, "    <coordinateInfo>")?;
    writeln!(buf, "      <fbd><scaling x=\"1\" y=\"1\"/></fbd>")?;
    writeln!(buf, "      <ld><scaling x=\"1\" y=\"1\"/></ld>")?;
    writeln!(buf, "      <sfc><scaling x=\"1\" y=\"1\"/></sfc>")?;
    writeln!(buf, "    </coordinateInfo>")?;
    writeln!(buf, "  </contentHeader>")?;
    writeln!(buf, "  <types>")?;
    writeln!(buf, "    <dataTypes/>")?;
    writeln!(buf, "    <pous>")?;

    for program in &project.programs {
        write_program(program, &mut buf)?;
    }

    writeln!(buf, "    </pous>")?;
    writeln!(buf, "  </types>")?;
    writeln!(buf, "  <instances>")?;
    writeln!(buf, "    <configurations>")?;
    writeln!(buf, "      <configuration name=\"Config\">")?;
    writeln!(buf, "        <resource name=\"Res\">")?;
    writeln!(
        buf,
        "          <task name=\"Main\" priority=\"0\" interval=\"T#20ms\">"
    )?;

    if let Some(prog) = project.programs.first() {
        writeln!(
            buf,
            "            <pouInstance name=\"{}\" typeName=\"{}\"/>",
            xml_escape(&prog.name),
            xml_escape(&prog.name)
        )?;
    }

    writeln!(buf, "          </task>")?;
    writeln!(buf, "        </resource>")?;
    writeln!(buf, "      </configuration>")?;
    writeln!(buf, "    </configurations>")?;
    writeln!(buf, "  </instances>")?;
    writeln!(buf, "</project>")?;

    Ok(buf)
}

fn write_program(program: &Program, buf: &mut String) -> Result<(), WriteError> {
    writeln!(
        buf,
        "      <pou name=\"{}\" pouType=\"program\">",
        xml_escape(&program.name)
    )?;

    // Write interface (variable declarations)
    writeln!(buf, "        <interface>")?;
    writeln!(buf, "          <localVars>")?;

    // Collect unique variables
    let mut vars = Vec::new();
    for rung in &program.rungs {
        for elem in &rung.elements {
            match elem {
                RungElement::Contact(c) => {
                    if !vars.contains(&c.variable) {
                        vars.push(c.variable.clone());
                    }
                }
                RungElement::Coil(c) => {
                    if !vars.contains(&c.variable) {
                        vars.push(c.variable.clone());
                    }
                }
                RungElement::Block(b) => {
                    if !vars.contains(&b.instance_name) {
                        vars.push(b.instance_name.clone());
                    }
                }
            }
        }
    }

    for var in &vars {
        writeln!(
            buf,
            "            <variable name=\"{}\"><type><BOOL/></type></variable>",
            xml_escape(var)
        )?;
    }

    writeln!(buf, "          </localVars>")?;
    writeln!(buf, "        </interface>")?;

    // Write body
    writeln!(buf, "        <body>")?;
    writeln!(buf, "          <LD>")?;

    // Start power rail IDs after the max existing element ID to avoid conflicts
    let max_elem_id = program
        .rungs
        .iter()
        .flat_map(|r| r.elements.iter())
        .map(|e| match e {
            RungElement::Contact(c) => c.local_id,
            RungElement::Coil(c) => c.local_id,
            RungElement::Block(b) => b.local_id,
        })
        .max()
        .unwrap_or(0);
    let mut next_local_id = max_elem_id + 1;
    let y_spacing = 60i32;

    for (rung_idx, rung) in program.rungs.iter().enumerate() {
        let rung_y = rung_idx as i32 * y_spacing * 2;
        write_rung(rung, buf, &mut next_local_id, rung_y)?;
    }

    writeln!(buf, "          </LD>")?;
    writeln!(buf, "        </body>")?;
    writeln!(buf, "      </pou>")?;

    Ok(())
}

fn write_rung(
    rung: &Rung,
    buf: &mut String,
    next_id: &mut u32,
    base_y: i32,
) -> Result<(), WriteError> {
    // We need to map old localIds to new ones.
    // The rung's elements have their original localIds; we'll reuse them as-is
    // and just need to emit a leftPowerRail whose localId connects to the first elements.

    // Find which elements connect from a power rail (have no incoming connection from another element in this rung)
    let element_ids: std::collections::HashSet<u32> = rung
        .elements
        .iter()
        .map(|e| match e {
            RungElement::Contact(c) => c.local_id,
            RungElement::Coil(c) => c.local_id,
            RungElement::Block(b) => b.local_id,
        })
        .collect();

    // Build incoming connections map
    let mut incoming: std::collections::HashMap<u32, Vec<u32>> = std::collections::HashMap::new();
    for conn in &rung.connections {
        incoming.entry(conn.to_id).or_default().push(conn.from_id);
    }

    // Elements whose inputs come from outside the rung (i.e., from power rail)
    let rail_connected: Vec<u32> = rung
        .elements
        .iter()
        .filter_map(|e| {
            let id = match e {
                RungElement::Contact(c) => c.local_id,
                RungElement::Coil(c) => c.local_id,
                RungElement::Block(b) => b.local_id,
            };
            let inputs = incoming.get(&id);
            match inputs {
                None => Some(id), // no incoming connections at all
                Some(ids) => {
                    if ids.iter().all(|ref_id| !element_ids.contains(ref_id)) {
                        Some(id) // all inputs are from outside (power rail)
                    } else {
                        None
                    }
                }
            }
        })
        .collect();

    // Emit leftPowerRail
    let rail_id = *next_id;
    *next_id += 1;

    writeln!(
        buf,
        "            <leftPowerRail localId=\"{}\">",
        rail_id
    )?;
    writeln!(buf, "              <position x=\"0\" y=\"{}\"/>", base_y)?;

    let conn_count = rail_connected.len().max(1);
    for i in 0..conn_count {
        writeln!(buf, "              <connectionPointOut formalParameter=\"\">")?;
        writeln!(
            buf,
            "                <relPosition x=\"0\" y=\"{}\"/>",
            i as i32 * 40
        )?;
        writeln!(buf, "              </connectionPointOut>")?;
    }
    writeln!(buf, "            </leftPowerRail>")?;

    // Sort elements by position: contacts first (by localId), then blocks, then coils
    let mut sorted_elements: Vec<&RungElement> = rung.elements.iter().collect();
    sorted_elements.sort_by_key(|e| match e {
        RungElement::Contact(c) => (0, c.local_id),
        RungElement::Block(b) => (1, b.local_id),
        RungElement::Coil(c) => (2, c.local_id),
    });

    let x_spacing = 80i32;

    for (i, elem) in sorted_elements.iter().enumerate() {
        let x = (i as i32 + 1) * x_spacing;

        match elem {
            RungElement::Contact(c) => {
                let negated = matches!(c.contact_type, ContactType::NormallyClosed);
                writeln!(
                    buf,
                    "            <contact localId=\"{}\" negated=\"{}\">",
                    c.local_id, negated
                )?;
                writeln!(
                    buf,
                    "              <position x=\"{}\" y=\"{}\"/>",
                    x, base_y
                )?;
                writeln!(buf, "              <connectionPointIn>")?;
                writeln!(buf, "                <relPosition x=\"0\" y=\"0\"/>")?;

                // Find what this element connects from
                if let Some(refs) = incoming.get(&c.local_id) {
                    for &ref_id in refs {
                        let actual_ref = if element_ids.contains(&ref_id) {
                            ref_id
                        } else {
                            rail_id
                        };
                        writeln!(
                            buf,
                            "                <connection refLocalId=\"{}\"/>",
                            actual_ref
                        )?;
                    }
                } else {
                    writeln!(
                        buf,
                        "                <connection refLocalId=\"{}\"/>",
                        rail_id
                    )?;
                }

                writeln!(buf, "              </connectionPointIn>")?;
                writeln!(buf, "              <connectionPointOut>")?;
                writeln!(buf, "                <relPosition x=\"60\" y=\"0\"/>")?;
                writeln!(buf, "              </connectionPointOut>")?;
                writeln!(
                    buf,
                    "              <variable>{}</variable>",
                    xml_escape(&c.variable)
                )?;
                writeln!(buf, "            </contact>")?;
            }
            RungElement::Coil(c) => {
                let storage = match c.coil_type {
                    CoilType::Set => " storage=\"set\"",
                    CoilType::Reset => " storage=\"reset\"",
                    CoilType::Normal => "",
                };
                writeln!(
                    buf,
                    "            <coil localId=\"{}\" negated=\"false\"{}>",
                    c.local_id, storage
                )?;
                writeln!(
                    buf,
                    "              <position x=\"{}\" y=\"{}\"/>",
                    x, base_y
                )?;
                writeln!(buf, "              <connectionPointIn>")?;
                writeln!(buf, "                <relPosition x=\"0\" y=\"0\"/>")?;

                if let Some(refs) = incoming.get(&c.local_id) {
                    for &ref_id in refs {
                        let actual_ref = if element_ids.contains(&ref_id) {
                            ref_id
                        } else {
                            rail_id
                        };
                        writeln!(
                            buf,
                            "                <connection refLocalId=\"{}\"/>",
                            actual_ref
                        )?;
                    }
                } else {
                    writeln!(
                        buf,
                        "                <connection refLocalId=\"{}\"/>",
                        rail_id
                    )?;
                }

                writeln!(buf, "              </connectionPointIn>")?;
                writeln!(buf, "              <connectionPointOut>")?;
                writeln!(buf, "                <relPosition x=\"60\" y=\"0\"/>")?;
                writeln!(buf, "              </connectionPointOut>")?;
                writeln!(
                    buf,
                    "              <variable>{}</variable>",
                    xml_escape(&c.variable)
                )?;
                writeln!(buf, "            </coil>")?;
            }
            RungElement::Block(b) => {
                let instance_attr = if b.instance_name.is_empty() {
                    String::new()
                } else {
                    format!(" instanceName=\"{}\"", xml_escape(&b.instance_name))
                };
                writeln!(
                    buf,
                    "            <block localId=\"{}\" typeName=\"{}\"{}>",
                    b.local_id,
                    xml_escape(&b.type_name),
                    instance_attr
                )?;
                writeln!(
                    buf,
                    "              <position x=\"{}\" y=\"{}\"/>",
                    x, base_y
                )?;
                writeln!(buf, "              <inputVariables>")?;

                for (param_name, _param_value) in &b.parameters {
                    writeln!(
                        buf,
                        "                <variable formalParameter=\"{}\">",
                        xml_escape(param_name)
                    )?;
                    writeln!(buf, "                  <connectionPointIn>")?;
                    writeln!(buf, "                    <relPosition x=\"0\" y=\"0\"/>")?;

                    // Only connect the first parameter (IN) to upstream
                    if param_name == "IN" {
                        if let Some(refs) = incoming.get(&b.local_id) {
                            for &ref_id in refs {
                                let actual_ref = if element_ids.contains(&ref_id) {
                                    ref_id
                                } else {
                                    rail_id
                                };
                                writeln!(
                                    buf,
                                    "                    <connection refLocalId=\"{}\"/>",
                                    actual_ref
                                )?;
                            }
                        }
                    }

                    writeln!(buf, "                  </connectionPointIn>")?;
                    writeln!(buf, "                </variable>")?;
                }

                writeln!(buf, "              </inputVariables>")?;
                writeln!(buf, "              <inOutVariables/>")?;
                writeln!(buf, "              <outputVariables>")?;
                writeln!(buf, "                <variable formalParameter=\"Q\">")?;
                writeln!(buf, "                  <connectionPointOut>")?;
                writeln!(buf, "                    <relPosition x=\"80\" y=\"0\"/>")?;
                writeln!(buf, "                  </connectionPointOut>")?;
                writeln!(buf, "                </variable>")?;
                writeln!(buf, "                <variable formalParameter=\"ET\">")?;
                writeln!(buf, "                  <connectionPointOut>")?;
                writeln!(buf, "                    <relPosition x=\"80\" y=\"30\"/>")?;
                writeln!(buf, "                  </connectionPointOut>")?;
                writeln!(buf, "                </variable>")?;
                writeln!(buf, "              </outputVariables>")?;
                writeln!(buf, "            </block>")?;
            }
        }
    }

    Ok(())
}

fn xml_escape(s: &str) -> String {
    s.replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
        .replace('"', "&quot;")
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::parser;

    fn fixture(name: &str) -> String {
        let path = format!("../../tests/fixtures/{name}");
        std::fs::read_to_string(&path).unwrap()
    }

    #[test]
    fn write_produces_valid_xml() {
        let project = parser::parse(&fixture("self_hold.xml")).unwrap();
        let xml = write(&project).unwrap();

        // Verify it's valid XML by parsing with quick-xml
        let mut reader = quick_xml::Reader::from_str(&xml);
        reader.config_mut().trim_text(true);
        loop {
            match reader.read_event() {
                Ok(quick_xml::events::Event::Eof) => break,
                Err(e) => panic!("Generated XML is not well-formed: {e}"),
                _ => {}
            }
        }
    }

    #[test]
    fn write_contains_project_structure() {
        let project = parser::parse(&fixture("interlock.xml")).unwrap();
        let xml = write(&project).unwrap();

        assert!(xml.contains("<project"));
        assert!(xml.contains("plcopen.org/xml/tc6_0201"));
        assert!(xml.contains("<LD>"));
        assert!(xml.contains("<leftPowerRail"));
        assert!(xml.contains("<contact"));
        assert!(xml.contains("<coil"));
        assert!(xml.contains("X001"));
        assert!(xml.contains("negated=\"true\""));
    }
}

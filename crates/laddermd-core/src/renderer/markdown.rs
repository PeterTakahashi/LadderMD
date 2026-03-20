#![allow(clippy::only_used_in_recursion)]

use std::collections::HashMap;
use std::fmt::Write as FmtWrite;
use std::io;

use crate::model::*;

/// Markdown renderer configuration.
pub struct MarkdownRenderer {
    pub render_diagram: bool,
    pub render_device_table: bool,
    pub render_logic: bool,
}

impl Default for MarkdownRenderer {
    fn default() -> Self {
        Self {
            render_diagram: true,
            render_device_table: true,
            render_logic: true,
        }
    }
}

impl MarkdownRenderer {
    pub fn render(&self, project: &Project) -> String {
        let mut buf = String::new();
        self.render_to_string(project, &mut buf);
        buf
    }

    pub fn render_to_writer(
        &self,
        project: &Project,
        w: &mut impl io::Write,
    ) -> io::Result<()> {
        let s = self.render(project);
        w.write_all(s.as_bytes())
    }

    fn render_to_string(&self, project: &Project, buf: &mut String) {
        writeln!(buf, "# Project: {}\n", project.name).unwrap();

        for program in &project.programs {
            writeln!(buf, "## Program: {}\n", program.name).unwrap();

            for (i, rung) in program.rungs.iter().enumerate() {
                if let Some(ref comment) = rung.comment {
                    writeln!(buf, "### Rung {}: {}\n", i + 1, comment).unwrap();
                } else {
                    writeln!(buf, "### Rung {}\n", i + 1).unwrap();
                }

                if self.render_logic {
                    self.render_logic_expression(rung, buf);
                }

                if self.render_device_table {
                    self.render_table(rung, buf);
                }

                if self.render_diagram {
                    self.render_ascii_diagram(rung, buf);
                }
            }
        }
    }

    fn render_logic_expression(&self, rung: &Rung, buf: &mut String) {
        // Build connection graph: for each element, what are its inputs?
        let mut inputs: HashMap<u32, Vec<u32>> = HashMap::new();
        for conn in &rung.connections {
            inputs.entry(conn.to_id).or_default().push(conn.from_id);
        }

        // Find coils (outputs)
        let coils: Vec<&Coil> = rung
            .elements
            .iter()
            .filter_map(|e| match e {
                RungElement::Coil(c) => Some(c),
                _ => None,
            })
            .collect();

        // Build element lookup
        let elem_by_id: HashMap<u32, &RungElement> = rung
            .elements
            .iter()
            .map(|e| {
                let id = match e {
                    RungElement::Contact(c) => c.local_id,
                    RungElement::Coil(c) => c.local_id,
                    RungElement::Block(b) => b.local_id,
                };
                (id, e)
            })
            .collect();

        for coil in &coils {
            let coil_type_str = match coil.coil_type {
                CoilType::Normal => "",
                CoilType::Set => " (SET)",
                CoilType::Reset => " (RESET)",
            };

            let expr = self.build_expression(coil.local_id, &inputs, &elem_by_id);
            writeln!(buf, "LOGIC: {}{} = {}", coil.variable, coil_type_str, expr).unwrap();
        }

        // Find blocks (function blocks like TON)
        let blocks: Vec<&Block> = rung
            .elements
            .iter()
            .filter_map(|e| match e {
                RungElement::Block(b) => Some(b),
                _ => None,
            })
            .collect();

        for block in &blocks {
            let expr = self.build_expression(block.local_id, &inputs, &elem_by_id);
            writeln!(
                buf,
                "LOGIC: {}({}) IN = {}",
                block.type_name, block.instance_name, expr
            )
            .unwrap();
        }

        if !coils.is_empty() || !blocks.is_empty() {
            writeln!(buf).unwrap();
        }
    }

    fn build_expression(
        &self,
        element_id: u32,
        inputs: &HashMap<u32, Vec<u32>>,
        elem_by_id: &HashMap<u32, &RungElement>,
    ) -> String {
        let input_ids = match inputs.get(&element_id) {
            Some(ids) => ids,
            None => return String::from("(power rail)"),
        };

        let sub_exprs: Vec<String> = input_ids
            .iter()
            .map(|&id| {
                if let Some(elem) = elem_by_id.get(&id) {
                    match elem {
                        RungElement::Contact(c) => {
                            let var = &c.variable;
                            let upstream = self.build_expression(c.local_id, inputs, elem_by_id);
                            let contact_str = match c.contact_type {
                                ContactType::NormallyOpen => var.to_string(),
                                ContactType::NormallyClosed => format!("NOT {var}"),
                            };
                            if upstream == "(power rail)" {
                                contact_str
                            } else {
                                format!("{upstream} AND {contact_str}")
                            }
                        }
                        RungElement::Block(b) => {
                            format!("{}.Q", b.instance_name)
                        }
                        RungElement::Coil(c) => c.variable.clone(),
                    }
                } else {
                    // Reference to power rail or unknown element
                    String::from("(power rail)")
                }
            })
            .collect();

        if sub_exprs.len() == 1 {
            sub_exprs.into_iter().next().unwrap()
        } else {
            format!("({})", sub_exprs.join(" OR "))
        }
    }

    fn render_table(&self, rung: &Rung, buf: &mut String) {
        writeln!(buf, "| Device | Type | LocalId |").unwrap();
        writeln!(buf, "|--------|------|---------|").unwrap();

        for elem in &rung.elements {
            match elem {
                RungElement::Contact(c) => {
                    let type_str = match c.contact_type {
                        ContactType::NormallyOpen => "Contact(NO)",
                        ContactType::NormallyClosed => "Contact(NC)",
                    };
                    writeln!(buf, "| {} | {} | {} |", c.variable, type_str, c.local_id).unwrap();
                }
                RungElement::Coil(c) => {
                    let type_str = match c.coil_type {
                        CoilType::Normal => "Coil",
                        CoilType::Set => "Coil(S)",
                        CoilType::Reset => "Coil(R)",
                    };
                    writeln!(buf, "| {} | {} | {} |", c.variable, type_str, c.local_id).unwrap();
                }
                RungElement::Block(b) => {
                    writeln!(
                        buf,
                        "| {} | Block({}) | {} |",
                        b.instance_name, b.type_name, b.local_id
                    )
                    .unwrap();
                }
            }
        }
        writeln!(buf).unwrap();
    }

    fn render_ascii_diagram(&self, rung: &Rung, buf: &mut String) {
        // Build connection graph to determine series/parallel structure
        let mut inputs: HashMap<u32, Vec<u32>> = HashMap::new();
        for conn in &rung.connections {
            inputs.entry(conn.to_id).or_default().push(conn.from_id);
        }

        // Find coils/blocks (terminal elements)
        let terminals: Vec<&RungElement> = rung
            .elements
            .iter()
            .filter(|e| matches!(e, RungElement::Coil(_) | RungElement::Block(_)))
            .collect();

        if terminals.is_empty() {
            return;
        }

        writeln!(buf, "```").unwrap();

        for terminal in &terminals {
            let terminal_id = match terminal {
                RungElement::Coil(c) => c.local_id,
                RungElement::Block(b) => b.local_id,
                _ => continue,
            };

            // Collect all paths from power rail to this terminal
            let paths = self.find_all_paths(terminal_id, &inputs, rung);

            let terminal_str = match terminal {
                RungElement::Coil(c) => {
                    let label = match c.coil_type {
                        CoilType::Normal => c.variable.clone(),
                        CoilType::Set => format!("S {}", c.variable),
                        CoilType::Reset => format!("R {}", c.variable),
                    };
                    format!("({})", label)
                }
                RungElement::Block(b) => format!("[{} {}]", b.type_name, b.instance_name),
                _ => String::new(),
            };

            if paths.len() == 1 {
                let mut line = String::from("|");
                for contact_str in &paths[0] {
                    write!(line, "--{contact_str}").unwrap();
                }
                write!(line, "--{terminal_str}|").unwrap();
                writeln!(buf, "{line}").unwrap();
            } else {
                // Multiple parallel paths (OR branches)
                for (j, path) in paths.iter().enumerate() {
                    let mut line = String::from("|");
                    for contact_str in path {
                        write!(line, "--{contact_str}").unwrap();
                    }
                    if j == 0 {
                        write!(line, "--+--{terminal_str}|").unwrap();
                    } else if j == paths.len() - 1 {
                        write!(line, "--+").unwrap();
                        // pad to align
                        let pad = terminal_str.len() + 2;
                        for _ in 0..pad {
                            line.push(' ');
                        }
                        line.push('|');
                    } else {
                        write!(line, "--+").unwrap();
                        let pad = terminal_str.len() + 2;
                        for _ in 0..pad {
                            line.push(' ');
                        }
                        line.push('|');
                    }
                    writeln!(buf, "{line}").unwrap();
                }
            }
        }

        writeln!(buf, "```\n").unwrap();
    }

    /// Find all paths from power rail to the given element, returning
    /// contact labels along each path.
    fn find_all_paths(
        &self,
        target_id: u32,
        inputs: &HashMap<u32, Vec<u32>>,
        rung: &Rung,
    ) -> Vec<Vec<String>> {
        let elem_by_id: HashMap<u32, &RungElement> = rung
            .elements
            .iter()
            .map(|e| {
                let id = match e {
                    RungElement::Contact(c) => c.local_id,
                    RungElement::Coil(c) => c.local_id,
                    RungElement::Block(b) => b.local_id,
                };
                (id, e)
            })
            .collect();

        self.trace_paths(target_id, inputs, &elem_by_id)
    }

    fn trace_paths(
        &self,
        element_id: u32,
        inputs: &HashMap<u32, Vec<u32>>,
        elem_by_id: &HashMap<u32, &RungElement>,
    ) -> Vec<Vec<String>> {
        let input_ids = match inputs.get(&element_id) {
            Some(ids) => ids,
            None => return vec![vec![]], // reached power rail
        };

        let mut all_paths = Vec::new();

        for &id in input_ids {
            let label = if let Some(elem) = elem_by_id.get(&id) {
                match elem {
                    RungElement::Contact(c) => {
                        let var = &c.variable;
                        match c.contact_type {
                            ContactType::NormallyOpen => format!("[{var}]"),
                            ContactType::NormallyClosed => format!("[/{var}]"),
                        }
                    }
                    RungElement::Block(b) => format!("[{} {}]", b.type_name, b.instance_name),
                    RungElement::Coil(c) => format!("({})", c.variable),
                }
            } else {
                continue; // power rail reference
            };

            let upstream_paths = self.trace_paths(id, inputs, elem_by_id);
            for mut path in upstream_paths {
                path.push(label.clone());
                all_paths.push(path);
            }
        }

        if all_paths.is_empty() {
            vec![vec![]]
        } else {
            all_paths
        }
    }
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
    fn render_self_hold() {
        let project = parser::parse(&fixture("self_hold.xml")).unwrap();
        let renderer = MarkdownRenderer::default();
        let md = renderer.render(&project);

        assert!(md.contains("# Project: SelfHoldTest"));
        assert!(md.contains("## Program: Main"));
        assert!(md.contains("### Rung 1"));
        assert!(md.contains("### Rung 2"));
        assert!(md.contains("LOGIC:"));
        assert!(md.contains("Contact(NO)"));
        assert!(md.contains("Coil(R)"));
        assert!(md.contains("[X001]"));
        assert!(md.contains("(Y001)"));
    }

    #[test]
    fn render_interlock() {
        let project = parser::parse(&fixture("interlock.xml")).unwrap();
        let renderer = MarkdownRenderer::default();
        let md = renderer.render(&project);

        assert!(md.contains("Contact(NC)"));
        assert!(md.contains("[/X002]"));
        assert!(md.contains("[/X001]"));
    }

    #[test]
    fn render_timer() {
        let project = parser::parse(&fixture("timer.xml")).unwrap();
        let renderer = MarkdownRenderer::default();
        let md = renderer.render(&project);

        assert!(md.contains("Block(TON)"));
        assert!(md.contains("T001_Instance"));
    }

    #[test]
    fn render_emergency_stop() {
        let project = parser::parse(&fixture("emergency_stop.xml")).unwrap();
        let renderer = MarkdownRenderer::default();
        let md = renderer.render(&project);

        assert!(md.contains("[/X010]"));
        assert!(md.contains("(Y001)"));
    }

    #[test]
    fn render_to_writer_works() {
        let project = parser::parse(&fixture("self_hold.xml")).unwrap();
        let renderer = MarkdownRenderer::default();
        let mut buf = Vec::new();
        renderer.render_to_writer(&project, &mut buf).unwrap();
        let s = String::from_utf8(buf).unwrap();
        assert!(s.contains("# Project: SelfHoldTest"));
    }

    #[test]
    fn render_options_can_disable_sections() {
        let project = parser::parse(&fixture("self_hold.xml")).unwrap();
        let renderer = MarkdownRenderer {
            render_diagram: false,
            render_device_table: false,
            render_logic: true,
        };
        let md = renderer.render(&project);
        assert!(md.contains("LOGIC:"));
        assert!(!md.contains("| Device |"));
        assert!(!md.contains("```"));
    }
}

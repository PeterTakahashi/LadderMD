use laddermd_core::{parser, renderer::MarkdownRenderer, validator, writer};
use pyo3::prelude::*;

/// Convert PLCopen XML to Markdown.
#[pyfunction]
#[pyo3(signature = (xml, *, no_diagram=false, no_table=false, no_logic=false))]
fn convert_xml(
    xml: &str,
    no_diagram: bool,
    no_table: bool,
    no_logic: bool,
) -> PyResult<String> {
    let project = parser::parse(xml)
        .map_err(|e| PyErr::new::<pyo3::exceptions::PyValueError, _>(format!("{e}")))?;
    let renderer = MarkdownRenderer {
        render_diagram: !no_diagram,
        render_device_table: !no_table,
        render_logic: !no_logic,
    };
    Ok(renderer.render(&project))
}

/// Convert Mitsubishi mnemonic format to Markdown.
#[pyfunction]
#[pyo3(signature = (mnemonic, *, no_diagram=false, no_table=false, no_logic=false))]
fn convert_mnemonic(
    mnemonic: &str,
    no_diagram: bool,
    no_table: bool,
    no_logic: bool,
) -> PyResult<String> {
    let project = parser::parse_mnemonic(mnemonic, None, None)
        .map_err(|e| PyErr::new::<pyo3::exceptions::PyValueError, _>(format!("{e}")))?;
    let renderer = MarkdownRenderer {
        render_diagram: !no_diagram,
        render_device_table: !no_table,
        render_logic: !no_logic,
    };
    Ok(renderer.render(&project))
}

/// Convert Mitsubishi mnemonic to PLCopen XML.
#[pyfunction]
fn mnemonic_to_xml(mnemonic: &str) -> PyResult<String> {
    let project = parser::parse_mnemonic(mnemonic, None, None)
        .map_err(|e| PyErr::new::<pyo3::exceptions::PyValueError, _>(format!("{e}")))?;
    writer::write(&project)
        .map_err(|e| PyErr::new::<pyo3::exceptions::PyRuntimeError, _>(format!("{e}")))
}

/// Convert PLCopen XML to Mitsubishi mnemonic format.
#[pyfunction]
fn xml_to_mnemonic(xml: &str) -> PyResult<String> {
    let project = parser::parse(xml)
        .map_err(|e| PyErr::new::<pyo3::exceptions::PyValueError, _>(format!("{e}")))?;
    Ok(parser::write_mnemonic(&project))
}

/// Validate roundtrip conversion of PLCopen XML.
/// Returns a dict with parse_ok, roundtrip_ok, total_rungs, contacts, coils, blocks, errors.
#[pyfunction]
fn validate(xml: &str) -> PyResult<PyObject> {
    let result = validator::validate(xml)
        .map_err(|e| PyErr::new::<pyo3::exceptions::PyValueError, _>(format!("{e}")))?;

    Python::with_gil(|py| {
        let dict = pyo3::types::PyDict::new(py);
        dict.set_item("parse_ok", result.parse_ok)?;
        dict.set_item("roundtrip_ok", result.roundtrip_ok)?;
        dict.set_item("total_rungs", result.total_rungs)?;
        dict.set_item("contacts", result.contacts)?;
        dict.set_item("coils", result.coils)?;
        dict.set_item("blocks", result.blocks)?;
        dict.set_item("errors", result.errors)?;
        Ok(dict.into())
    })
}

/// Get project info from PLCopen XML or mnemonic.
#[pyfunction]
#[pyo3(signature = (input, format="xml"))]
fn info(input: &str, format: &str) -> PyResult<PyObject> {
    let project = match format {
        "mnemonic" | "mn" => parser::parse_mnemonic(input, None, None)
            .map_err(|e| PyErr::new::<pyo3::exceptions::PyValueError, _>(format!("{e}")))?,
        _ => parser::parse(input)
            .map_err(|e| PyErr::new::<pyo3::exceptions::PyValueError, _>(format!("{e}")))?,
    };

    Python::with_gil(|py| {
        let dict = pyo3::types::PyDict::new(py);
        dict.set_item("project_name", &project.name)?;

        let programs: Vec<PyObject> = project
            .programs
            .iter()
            .map(|prog| {
                let pdict = pyo3::types::PyDict::new(py);
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
                pdict.set_item("name", &prog.name).unwrap();
                pdict.set_item("rungs", prog.rungs.len()).unwrap();
                pdict.set_item("contacts", contacts).unwrap();
                pdict.set_item("coils", coils).unwrap();
                pdict.set_item("blocks", blocks).unwrap();
                pdict.into()
            })
            .collect();
        dict.set_item("programs", programs)?;
        Ok(dict.into())
    })
}

/// LadderMD: PLC ladder diagram converter.
///
/// Convert PLCopen XML and Mitsubishi mnemonic format to Markdown.
#[pymodule]
fn laddermd(m: &Bound<'_, PyModule>) -> PyResult<()> {
    m.add_function(wrap_pyfunction!(convert_xml, m)?)?;
    m.add_function(wrap_pyfunction!(convert_mnemonic, m)?)?;
    m.add_function(wrap_pyfunction!(mnemonic_to_xml, m)?)?;
    m.add_function(wrap_pyfunction!(xml_to_mnemonic, m)?)?;
    m.add_function(wrap_pyfunction!(validate, m)?)?;
    m.add_function(wrap_pyfunction!(info, m)?)?;
    Ok(())
}

use quick_xml::events::Event;
use quick_xml::Reader;
use std::fs;
fn assert_wellformed_xml(path: &str) {
    let xml = fs::read_to_string(path).unwrap_or_else(|e| panic!("Failed to read {path}: {e}"));
    let mut reader = Reader::from_str(&xml);
    reader.config_mut().trim_text(true);

    let mut depth = 0i32;
    loop {
        match reader.read_event() {
            Ok(Event::Start(_)) => depth += 1,
            Ok(Event::End(_)) => {
                depth -= 1;
                assert!(depth >= 0, "{path}: mismatched closing tag");
            }
            Ok(Event::Eof) => break,
            Err(e) => panic!("{path}: XML parse error at position {}: {e}", reader.error_position()),
            _ => {}
        }
    }
    assert_eq!(depth, 0, "{path}: unclosed tags remain (depth={depth})");
}

#[test]
fn self_hold_is_wellformed() {
    assert_wellformed_xml("../../tests/fixtures/self_hold.xml");
}

#[test]
fn interlock_is_wellformed() {
    assert_wellformed_xml("../../tests/fixtures/interlock.xml");
}

#[test]
fn timer_is_wellformed() {
    assert_wellformed_xml("../../tests/fixtures/timer.xml");
}

#[test]
fn emergency_stop_is_wellformed() {
    assert_wellformed_xml("../../tests/fixtures/emergency_stop.xml");
}

#[test]
fn all_fixtures_have_project_root() {
    let fixtures = [
        "../../tests/fixtures/self_hold.xml",
        "../../tests/fixtures/interlock.xml",
        "../../tests/fixtures/timer.xml",
        "../../tests/fixtures/emergency_stop.xml",
        "../../tests/fixtures/counter.xml",
        "../../tests/fixtures/comparison.xml",
    ];
    for path in &fixtures {
        let xml = fs::read_to_string(path).unwrap();
        assert!(
            xml.contains("<project"),
            "{path}: missing <project> root element"
        );
        assert!(
            xml.contains("plcopen.org/xml/tc6_0201"),
            "{path}: missing PLCopen namespace"
        );
    }
}

#[test]
fn all_fixtures_have_ld_body() {
    let fixtures = [
        "../../tests/fixtures/self_hold.xml",
        "../../tests/fixtures/interlock.xml",
        "../../tests/fixtures/timer.xml",
        "../../tests/fixtures/emergency_stop.xml",
        "../../tests/fixtures/counter.xml",
        "../../tests/fixtures/comparison.xml",
    ];
    for path in &fixtures {
        let xml = fs::read_to_string(path).unwrap();
        assert!(xml.contains("<LD>"), "{path}: missing <LD> body element");
        assert!(
            xml.contains("<leftPowerRail"),
            "{path}: missing leftPowerRail"
        );
    }
}

#[test]
fn counter_is_wellformed() {
    assert_wellformed_xml("../../tests/fixtures/counter.xml");
}

#[test]
fn comparison_is_wellformed() {
    assert_wellformed_xml("../../tests/fixtures/comparison.xml");
}

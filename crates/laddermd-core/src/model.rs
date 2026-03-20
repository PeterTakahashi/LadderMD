use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum ContactType {
    NormallyOpen,
    NormallyClosed,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum CoilType {
    Normal,
    Set,
    Reset,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Contact {
    pub variable: String,
    pub contact_type: ContactType,
    pub local_id: u32,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Coil {
    pub variable: String,
    pub coil_type: CoilType,
    pub local_id: u32,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Block {
    pub type_name: String,
    pub instance_name: String,
    pub local_id: u32,
    pub parameters: Vec<(String, String)>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum RungElement {
    Contact(Contact),
    Coil(Coil),
    Block(Block),
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Connection {
    pub from_id: u32,
    pub to_id: u32,
}

/// A single rung (one horizontal line) of a ladder diagram.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Rung {
    pub comment: Option<String>,
    pub elements: Vec<RungElement>,
    pub connections: Vec<Connection>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Program {
    pub name: String,
    pub rungs: Vec<Rung>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Project {
    pub name: String,
    pub programs: Vec<Program>,
}

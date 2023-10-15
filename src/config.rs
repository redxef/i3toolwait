use std::fmt::{Display, Formatter};

use rust_lisp::model::Value as RValue;
use serde::{Deserialize, Deserializer};

#[derive(Clone, Debug)]
pub struct Value(Vec<RValue>);
unsafe impl Send for Value {}
unsafe impl Sync for Value {}

impl Into<Value> for RValue {
    fn into(self) -> Value {
        Value(vec![self])
    }
}

impl Into<Vec<RValue>> for Value {
    fn into(self) -> Vec<RValue> {
        self.0
    }
}

impl Display for Value {
    fn fmt(&self, f: &mut Formatter) -> std::fmt::Result {
        let mut s = String::new();
        s.push_str("(begin\n");
        for i in &self.0 {
            s.push_str(&format!("{}\n", i));
        }
        s.push_str(")");
        write!(f, "{}", &s)
    }
}

impl<'de> Deserialize<'de> for Value {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        let s: String = Deserialize::deserialize(deserializer)?;
        let r: Vec<RValue> = rust_lisp::parser::parse(&s)
            .filter_map(|x| x.ok())
            .collect();
        Ok(Value(r))
    }
}

#[derive(Clone, Debug, Deserialize)]
pub struct Program {
    #[serde(rename = "match")]
    pub match_: Value,
    pub cmd: String,
    #[serde(default)]
    pub run: Option<String>,
}

#[derive(Clone, Debug, Deserialize)]
pub struct Signal {
    #[serde(default)]
    pub run: Option<String>,
    #[serde(default = "Signal::default_timeout")]
    pub timeout: u64,
}
impl Signal {
    fn default_timeout() -> u64 {
        500
    }
}

#[derive(Clone, Debug, Deserialize)]
#[serde(untagged)]
pub enum ProgramEntry {
    Program(Program),
    Signal(Signal),
}

// Program is only unsafe because Value has dyn Any in it (via Foreign).
// if we don't use !Send in Foreign everything is fine.
unsafe impl Send for Program {}

#[derive(Clone, Debug, Deserialize)]
pub struct Config {
    #[serde(default = "Config::default_timeout")]
    pub timeout: u64,
    #[serde(default = "Config::default_init")]
    pub init: Value,
    #[serde(default = "Config::default_programs")]
    pub programs: Vec<ProgramEntry>,
    #[serde(default)]
    pub cmd: Option<String>,
}
// Config is only unsafe because Value has dyn Any in it (via Foreign).
// if we don't use !Send in Foreign everything is fine.
unsafe impl Send for Config {}
impl Config {
    fn default_timeout() -> u64 {
        3000
    }
    fn default_init() -> Value {
        Value(vec![])
    }
    fn default_programs() -> Vec<ProgramEntry> {
        vec![]
    }
}

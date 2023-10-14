use serde::{Serialize, Deserialize, Serializer, Deserializer};
use rust_lisp::model::Value as RValue;

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

impl<'de> Deserialize<'de> for Value {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>
    {
        let s: String = Deserialize::deserialize(deserializer)?;
        let r: Vec<RValue> = rust_lisp::parser::parse(&s).filter_map(|x| x.ok()).collect();
        Ok(Value(r))
    }
}


#[derive(Clone, Debug)]
#[derive(Deserialize)]
pub struct Program {
    #[serde(rename = "match")]
    pub match_: Value,
    pub cmd: String,
    #[serde(default)]
    pub run: Option<String>,
}
unsafe impl Send for Program {}
unsafe impl Sync for Program {}

#[derive(Clone, Debug)]
#[derive(Deserialize)]
pub struct Config {
    #[serde(default = "Config::default_timeout")]
    pub timeout: u32,
    #[serde(default = "Config::default_init")]
    pub init: Value,
    #[serde(default = "Config::default_programs")]
    pub programs: Vec<Program>,
}
unsafe impl Send for Config {}
unsafe impl Sync for Config {}
impl Config {
    fn default_timeout() -> u32 {
        3000
    }
    fn default_init() -> Value {
        Value(vec![])
    }
    fn default_programs() -> Vec<Program> {
        vec![]
    }
}

use std::collections::HashMap;

use rust_lisp::model::{reference, reference::Reference, Env, FloatType, IntType, List, Value};

fn serde_lisp_value(value: &serde_json::Value) -> Value {
    match value {
        serde_json::Value::Null => Value::NIL,
        serde_json::Value::Bool(b) => {
            if *b {
                Value::True
            } else {
                Value::False
            }
        }
        serde_json::Value::Number(n) => {
            if n.is_i64() {
                Value::Int(n.as_i64().unwrap() as IntType)
            } else if n.is_u64() {
                Value::Int(n.as_u64().unwrap() as IntType)
            } else if n.is_f64() {
                Value::Float(n.as_f64().unwrap() as FloatType)
            } else {
                panic!("should never happen");
            }
        }
        serde_json::Value::String(s) => Value::String(s.clone()),
        serde_json::Value::Array(a) => {
            let mut l = List::NIL;
            for li in a.into_iter().rev() {
                l = l.cons(serde_lisp_value(li));
            }
            Value::List(l)
        }
        serde_json::Value::Object(o) => {
            let mut r = HashMap::new();
            for (k, v) in o.into_iter() {
                let k_ = Value::String(k.clone());
                let v_ = serde_lisp_value(v);
                r.insert(k_, v_);
            }
            Value::HashMap(reference::new(r))
        }
    }
}

pub fn env(value: &serde_json::Value) -> Env {
    let mut environment = rust_lisp::default_env();
    environment.define(
        rust_lisp::model::Symbol::from("__input__"),
        serde_lisp_value(value),
    );
    environment.define(
        rust_lisp::model::Symbol::from("load"),
        rust_lisp::model::Value::NativeClosure(reference::new(
            move |e: Reference<rust_lisp::model::Env>, args: Vec<rust_lisp::model::Value>| {
                let path: &String =
                    rust_lisp::utils::require_typed_arg::<&String>("load", &args, 0)?;
                let path = (*path).as_str().split('.');
                let mut i: rust_lisp::model::Value = reference::borrow(&e)
                    .get(&rust_lisp::model::Symbol::from("__input__"))
                    .unwrap();
                for p in path
                    .into_iter()
                    .filter(|x| !(*x).eq(""))
                    .map(|x| rust_lisp::model::Value::String(x.into()))
                {
                    match i {
                        rust_lisp::model::Value::HashMap(x) => {
                            if let Some(_i) = reference::borrow(&x).get(&p) {
                                i = _i.clone();
                            } else {
                                return Err(rust_lisp::model::RuntimeError {
                                    msg: format!(r#"No such key {:?}"#, p).into(),
                                });
                            }
                        }
                        _ => {
                            return Err(rust_lisp::model::RuntimeError {
                                msg: format!(r#"No such key {:?}"#, p).into(),
                            })
                        }
                    };
                }
                Ok(i)
            },
        )),
    );
    environment.define(
        rust_lisp::model::Symbol::from("has-key"),
        rust_lisp::model::Value::NativeClosure(reference::new(
            move |e: Reference<rust_lisp::model::Env>, args: Vec<rust_lisp::model::Value>| {
                let path: &String =
                    rust_lisp::utils::require_typed_arg::<&String>("has-key", &args, 0)?;
                let path = (*path).as_str().split('.');
                let mut i: rust_lisp::model::Value = reference::borrow(&e)
                    .get(&rust_lisp::model::Symbol::from("__input__"))
                    .unwrap();
                for p in path
                    .into_iter()
                    .filter(|x| !(*x).eq(""))
                    .map(|x| rust_lisp::model::Value::String(x.into()))
                {
                    match i {
                        rust_lisp::model::Value::HashMap(x) => {
                            if let Some(_i) = reference::borrow(&x).get(&p) {
                                i = _i.clone();
                            } else {
                                return Ok(rust_lisp::model::Value::False);
                            }
                        }
                        _ => return Ok(rust_lisp::model::Value::False),
                    };
                }
                Ok(rust_lisp::model::Value::True)
            },
        )),
    );
    environment
}

use anyhow::Result;
use std::cell::RefCell;
use std::collections::HashMap;
use std::rc::Rc;
use tokio::time::{timeout, Duration};

mod i3ipc;

use i3ipc::{Connection, MessageType};

fn serde_lisp_value(value: &serde_json::Value) -> rust_lisp::model::Value {
    match value {
        serde_json::Value::Null => rust_lisp::model::Value::NIL,
        serde_json::Value::Bool(b) => {
            if *b {
                rust_lisp::model::Value::True
            } else {
                rust_lisp::model::Value::False
            }
        }
        serde_json::Value::Number(n) => {
            if n.is_i64() {
                rust_lisp::model::Value::Int(n.as_i64().unwrap() as rust_lisp::model::IntType)
            } else if n.is_u64() {
                rust_lisp::model::Value::Int(n.as_u64().unwrap() as rust_lisp::model::IntType)
            } else if n.is_f64() {
                rust_lisp::model::Value::Float(n.as_f64().unwrap() as rust_lisp::model::FloatType)
            } else {
                panic!("should never happen");
            }
        }
        serde_json::Value::String(s) => rust_lisp::model::Value::String(s.clone()),
        serde_json::Value::Array(a) => {
            let mut l = rust_lisp::model::List::NIL;
            for li in a.into_iter().rev() {
                l = l.cons(serde_lisp_value(li));
            }
            rust_lisp::model::Value::List(l)
        }
        serde_json::Value::Object(o) => {
            let mut r = HashMap::new();
            for (k, v) in o.into_iter() {
                let k_ = rust_lisp::model::Value::String(k.clone());
                let v_ = serde_lisp_value(v);
                r.insert(k_, v_);
            }
            rust_lisp::model::Value::HashMap(Rc::new(RefCell::new(r)))
        }
    }
}

fn new_window_cb(
    b: MessageType,
    c: serde_json::Value,
    d: bool,
) -> futures::future::BoxFuture<'static, Vec<(MessageType, Vec<u8>)>> {
    Box::pin(async move {
        let mut environment = rust_lisp::default_env();
        environment.define(
            rust_lisp::model::Symbol::from("__input__"),
            serde_lisp_value(&c),
        );
        environment.define(
            rust_lisp::model::Symbol::from("load"),
            rust_lisp::model::Value::NativeClosure(Rc::new(RefCell::new(
                move |e: Rc<RefCell<rust_lisp::model::Env>>, args: Vec<rust_lisp::model::Value>| {
                    let path: &String =
                        rust_lisp::utils::require_typed_arg::<&String>("load", &args, 0)?;
                    let path = (*path).as_str().split('.');
                    let mut i: rust_lisp::model::Value = e
                        .as_ref()
                        .borrow()
                        .get(&rust_lisp::model::Symbol::from("__input__"))
                        .unwrap();
                    for p in path
                        .into_iter()
                        .filter(|x| !(*x).eq(""))
                        .map(|x| rust_lisp::model::Value::String(x.into()))
                    {
                        match i {
                            rust_lisp::model::Value::HashMap(x) => {
                                if let Some(_i) = x.as_ref().borrow().get(&p) {
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
            ))),
        );
        environment.define(
            rust_lisp::model::Symbol::from("has-key"),
            rust_lisp::model::Value::NativeClosure(Rc::new(RefCell::new(
                move |e: Rc<RefCell<rust_lisp::model::Env>>, args: Vec<rust_lisp::model::Value>| {
                    let path: &String =
                        rust_lisp::utils::require_typed_arg::<&String>("has-key", &args, 0)?;
                    let path = (*path).as_str().split('.');
                    let mut i: rust_lisp::model::Value = e
                        .as_ref()
                        .borrow()
                        .get(&rust_lisp::model::Symbol::from("__input__"))
                        .unwrap();
                    for p in path
                        .into_iter()
                        .filter(|x| !(*x).eq(""))
                        .map(|x| rust_lisp::model::Value::String(x.into()))
                    {
                        match i {
                            rust_lisp::model::Value::HashMap(x) => {
                                if let Some(_i) = x.as_ref().borrow().get(&p) {
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
            ))),
        );
        let environment = environment;
        let code = r#"(load ".container.geometry")"#;
        let ast = rust_lisp::parser::parse(code).filter_map(|a| a.ok());
        let result = rust_lisp::interpreter::eval_block(Rc::new(RefCell::new(environment)), ast);
        println!("{:?}", result);

        Vec::new()
    })
}

async fn run<'a>(c: &mut Connection<'a>) -> Result<(), anyhow::Error> {
    let resp = c.communicate(&MessageType::Version, b"").await?;
    println!("{:?}", resp);

    c.communicate(&MessageType::Command, b"exec alacritty")
        .await?;
    Ok(())
}

#[tokio::main]
async fn main() -> Result<()> {
    let mut connection = Connection::connect((i3ipc::get_socket_path().await?).as_ref())?;
    let mut sub_connection = connection.clone();

    let b_ = true;
    let cb = move |a, b| {new_window_cb(a,b,b_)};
    sub_connection
        .subscribe(&[MessageType::SubWindow], &cb)
        .await?;
    tokio::join!(
        timeout(Duration::from_secs(1), sub_connection.run()),
        run(&mut connection),
    )
    .1?;
    Ok(())
}

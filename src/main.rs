use anyhow::Result;
use std::cell::RefCell;
use std::rc::Rc;
use tokio::time::{timeout, Duration};

mod i3ipc;

use i3ipc::{Connection, MessageType};

fn new_window_cb(
    b: MessageType,
    c: serde_json::Value,
) -> futures::future::BoxFuture<'static, Vec<(MessageType, Vec<u8>)>> {
    Box::pin(async move {
        //println!("{:?}", c);
        let environment = rust_lisp::default_env();
        let code = "(= 1 1)";
        let ast = rust_lisp::parser::parse(code).filter_map(|a| a.ok());
        let result = rust_lisp::interpreter::eval_block(Rc::new(RefCell::new(environment)), ast);

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

    sub_connection
        .subscribe(&[MessageType::SubWindow], &new_window_cb)
        .await?;
    tokio::join!(
        timeout(Duration::from_secs(1), sub_connection.run()),
        run(&mut connection),
    )
    .1?;
    Ok(())
}

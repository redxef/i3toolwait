use anyhow::Result;
use tokio::time::{timeout, Duration};
use tokio::io::AsyncReadExt;

mod config;
mod i3ipc;
mod lisp;

use config::Config;
use i3ipc::{Connection, MessageType};

fn new_window_cb(
    b: MessageType,
    c: serde_json::Value,
    config: &Config,
) -> futures::future::BoxFuture<'static, Vec<(MessageType, Vec<u8>)>> {
    let config_ = config.clone();
    Box::pin(async move {
        for p in config_.programs.iter() {
            let e = lisp::env(&c);
            let init: Vec<rust_lisp::model::Value> = config_.init.clone().into();
            let prog: Vec<rust_lisp::model::Value> = p.match_.clone().into();
            let m = init.into_iter().chain(prog.into_iter());
            let result = rust_lisp::interpreter::eval_block(rust_lisp::model::reference::new(e), m);
            println!("{:?}", result);
            if let Ok(rust_lisp::model::Value::True) = result {
                return vec![(MessageType::Command, p.cmd.clone().into_bytes())];
            }
        }

        Vec::new()
    })
}

async fn run<'a>(connection: &mut Connection<'a>, config: &Config) -> Result<(), anyhow::Error> {
    let resp = connection.communicate(&MessageType::Version, b"").await?;
    println!("{:?}", resp);

    for p in config.programs.iter() {
        if let Some(r) = &p.run {
            let (message_type, response) = connection.communicate(&MessageType::Command, r.as_bytes()).await?;
            println!("{:?}", (message_type, response));
        }
    }
    Ok(())
}

#[tokio::main]
async fn main() -> Result<()> {
    let mut connection = Connection::connect((i3ipc::get_socket_path().await?).as_ref())?;
    let mut sub_connection = connection.clone();

    let mut config = String::new();
    tokio::fs::File::open("/home/redxef/CODE/i3toolwait/i3_autostart.yaml").await?.read_to_string(&mut config).await?;
    let config: Config = serde_yaml::from_str(&config)?;
    let config = std::sync::Arc::new(config);

    let cb_config = config.clone();
    let cb = move |a, b| {new_window_cb(a, b, &cb_config)};
    sub_connection
        .subscribe(&[MessageType::SubWindow], &cb)
        .await?;

    tokio::join!(
        timeout(Duration::from_millis(config.timeout as u64), sub_connection.run()),
        run(&mut connection, &config),
    )
    .1?;
    Ok(())
}

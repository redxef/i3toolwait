use anyhow::Result;
use tokio::time::{timeout, Duration};
use tokio::io::AsyncReadExt;
use clap::Parser;
use log::{info, debug, warn};
use std::path::PathBuf;
use std::str::FromStr;

mod config;
mod i3ipc;
mod lisp;

use config::Config;
use i3ipc::{Connection, MessageType};


#[derive(Debug, Clone)]
#[derive(Parser)]
#[command(author, version, about, long_about = None)]
struct Args {
    #[arg(short, long, value_name = "FILE")]
    config: Option<PathBuf>,
}

impl Args {
    fn finish(&mut self) {
        // TODO maybe return separate type
        if self.config.is_none() {
            self.config = Some(xdg::BaseDirectories::with_prefix("i3toolwait").unwrap().get_config_file("config.yaml"));
        }
    }
}

fn new_window_cb(
    b: MessageType,
    c: serde_json::Value,
    config: &Config,
    args: &Args,
) -> futures::future::BoxFuture<'static, Vec<(MessageType, Vec<u8>)>> {
    let config_ = config.clone();
    Box::pin(async move {
        debug!("Received window event: {}", &c);
        for p in config_.programs.iter() {
            debug!("Evaluating program: {}", &p.match_);
            let e = lisp::env(&c);
            let init: Vec<rust_lisp::model::Value> = config_.init.clone().into();
            let prog: Vec<rust_lisp::model::Value> = p.match_.clone().into();
            let m = init.into_iter().chain(prog.into_iter());
            let result = rust_lisp::interpreter::eval_block(rust_lisp::model::reference::new(e), m);
            if let Ok(rust_lisp::model::Value::True) = result {
                debug!("Match found");
                return vec![(MessageType::Command, p.cmd.clone().into_bytes())];
            }
        }
        debug!("No match found");
        Vec::new()
    })
}

async fn run<'a>(connection: &mut Connection<'a>, config: &Config) -> Result<(), anyhow::Error> {
    let (_, resp) = connection.communicate(&MessageType::Version, b"").await?;
    info!("i3 version is {}", resp.get("human_readable").unwrap());

    for p in config.programs.iter() {
        if let Some(r) = &p.run {
            let (_, responses) = connection.communicate(&MessageType::Command, r.as_bytes()).await?;
            match responses {
                serde_json::Value::Array(responses) => {
                    for response in responses {
                        if let serde_json::Value::Bool(v) = response.get("success").unwrap() {
                            if !v {
                                warn!("Failed to run command {}: {}", r, response);
                            }
                        }
                    }
                },
                _ => panic!("invalid response"),
            };
        }
    }
    Ok(())
}

#[tokio::main]
async fn main() -> Result<()> {
    env_logger::init_from_env(env_logger::Env::new().filter("I3TOOLWAIT_LOG").write_style("I3TOOLWAIT_LOG_STYLE"));

    let mut args = Args::parse();
    args.finish();
    let args = std::sync::Arc::new(args);
    let mut config = String::new();
    if args.config.as_ref().unwrap() == &PathBuf::from_str("-").unwrap() {
        tokio::io::stdin().read_to_string(&mut config).await?;
    } else {
        tokio::fs::File::open(args.config.as_ref().unwrap()).await?.read_to_string(&mut config).await?;
    }
    let config: Config = serde_yaml::from_str(&config)?;
    let config = std::sync::Arc::new(config);

    let mut connection = Connection::connect((i3ipc::get_socket_path().await?).as_ref())?;
    let mut sub_connection = connection.clone();
    let cb_config = config.clone();
    let cb_args = args.clone();
    let cb = move |a, b| {new_window_cb(a, b, &cb_config, &cb_args)};
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

use std::collections::HashMap;
use std::path::PathBuf;
use std::str::FromStr;

use anyhow::Result;
use clap::Parser;
use log::{debug, info, warn};
use tokio::io::AsyncReadExt;
use tokio::time::{timeout, Duration};

mod config;
mod i3ipc;
mod lisp;

use config::{Config, ProgramEntry};
use i3ipc::{Connection, MessageType};

#[derive(Debug, Clone, Parser)]
#[command(author, version, about, long_about = None)]
struct Args {
    #[arg(short, long, value_name = "FILE")]
    config: Option<PathBuf>,
}

impl Args {
    fn finish(&mut self) {
        // TODO maybe return separate type
        if self.config.is_none() {
            self.config = Some(
                xdg::BaseDirectories::with_prefix("i3toolwait")
                    .unwrap()
                    .get_config_file("config.yaml"),
            );
        }
    }
}

fn new_window_cb(
    _b: MessageType,
    c: serde_json::Value,
    config: &Config,
    _args: &Args,
    programs: &std::sync::Arc<tokio::sync::Mutex<Vec<ProgramEntry>>>,
    tx: &tokio::sync::broadcast::Sender<()>,
) -> futures::future::BoxFuture<'static, Vec<(MessageType, Vec<u8>)>> {
    let config_ = config.clone();
    let tx_ = tx.clone();
    let programs_ = programs.clone();
    Box::pin(async move {
        let mut command = None;
        let mut index = None;
        debug!("Received window event: {}", &c);
        for (i, p) in programs_.lock().await.iter().enumerate() {
            match p {
                ProgramEntry::Program(p) => {
                    debug!("Evaluating program: {}", &p.match_);
                    let e = lisp::env(&c);
                    let init: Vec<rust_lisp::model::Value> = config_.init.clone().into();
                    let prog: Vec<rust_lisp::model::Value> = p.match_.clone().into();
                    let m = init.into_iter().chain(prog.into_iter());
                    let result =
                        rust_lisp::interpreter::eval_block(rust_lisp::model::reference::new(e), m);
                    if let Ok(v) = &result {
                        debug!("Received result: {}", v);
                        if *v == rust_lisp::model::Value::False {
                            continue;
                        }
                        debug!("Match found");
                        let mut vars = HashMap::with_capacity(1);
                        vars.insert("result".to_string(), v.to_string());
                        let cmd = strfmt::strfmt(&p.cmd, &vars).unwrap();
                        debug!("Command: {}", &cmd);

                        index = Some(i);
                        command = Some(cmd);
                        break;
                    } else {
                        warn!("Program produced an error: {:?}", &result);
                    }
                }
                _ => {
                    // Ignore signal entries
                    ()
                }
            };
        }
        if let Some(index) = index {
            let mut plock = programs_.lock().await;
            plock.remove(index);
            if plock.len() == 0 {
                tx_.send(()).unwrap();
            }
            return vec![(MessageType::Command, command.unwrap().into_bytes())];
        }
        debug!("No match found");
        Vec::new()
    })
}

async fn run_command<'a>(
    connection: &mut Connection<'a>,
    command: &str,
) -> Result<(), anyhow::Error> {
    let (_, responses) = connection
        .communicate(&MessageType::Command, command.as_bytes())
        .await?;
    match responses {
        serde_json::Value::Array(responses) => {
            for response in responses {
                if let serde_json::Value::Bool(v) = response.get("success").unwrap() {
                    if !v {
                        warn!("Failed to run command {}: {}", command, response);
                    }
                }
            }
        }
        _ => panic!("invalid response"),
    };
    Ok(())
}

async fn run<'a>(connection: &mut Connection<'a>, config: &Config) -> Result<(), anyhow::Error> {
    let (_, resp) = connection.communicate(&MessageType::Version, b"").await?;
    info!("i3 version is {}", resp.get("human_readable").unwrap());

    let mut signal_stream =
        tokio::signal::unix::signal(tokio::signal::unix::SignalKind::user_defined1())?;

    for p in config.programs.iter() {
        match p {
            ProgramEntry::Program(p) => {
                if let Some(r) = &p.run {
                    run_command(connection, r).await?;
                }
            }
            ProgramEntry::Signal(p) => {
                if let Some(r) = &p.run {
                    run_command(connection, r).await?;
                }
                if let Err(_) =
                    timeout(Duration::from_millis(p.timeout), signal_stream.recv()).await
                {
                    warn!(
                        "Ran into timeout when waiting for signal, program: {:?}",
                        p.run
                    );
                }
            }
        };
    }
    Ok(())
}

#[tokio::main]
async fn main() -> Result<()> {
    env_logger::init_from_env(
        env_logger::Env::new()
            .filter("I3TOOLWAIT_LOG")
            .write_style("I3TOOLWAIT_LOG_STYLE"),
    );

    let mut args = Args::parse();
    args.finish();
    let args = std::sync::Arc::new(args);
    let mut config = String::new();
    if args.config.as_ref().unwrap() == &PathBuf::from_str("-").unwrap() {
        tokio::io::stdin().read_to_string(&mut config).await?;
    } else {
        tokio::fs::File::open(args.config.as_ref().unwrap())
            .await?
            .read_to_string(&mut config)
            .await?;
    }
    let config: Config = serde_yaml::from_str(&config)?;
    let config = std::sync::Arc::new(config);
    let programs = std::sync::Arc::new(tokio::sync::Mutex::new(config.programs.clone()));

    let mut connection = Connection::connect((i3ipc::get_socket_path().await?).as_ref())?;
    let mut sub_connection = connection.clone();
    let cb_config = config.clone();
    let cb_args = args.clone();

    let (tx, mut rx) = tokio::sync::broadcast::channel::<()>(1);

    let cb_programs = programs.clone();
    let cb = move |a, b| new_window_cb(a, b, &cb_config, &cb_args, &cb_programs, &tx);
    sub_connection
        .subscribe(&[MessageType::SubWindow], &cb)
        .await?;

    tokio::join!(
        timeout(
            Duration::from_millis(config.timeout),
            sub_connection.run(&mut rx)
        ),
        run(&mut connection, &config),
    )
    .1?;
    {
        let p = programs.lock().await;
        if p.len() != 0 {
            warn!("Not all programs consumed: {:?}", &p);
            info!("Maybe the timouts are too short?");
        }
    }

    if let Some(cmd) = &config.cmd {
        connection
            .communicate(&MessageType::Command, cmd.as_bytes())
            .await?;
    }
    Ok(())
}

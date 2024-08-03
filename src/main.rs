mod cli;
mod server;
mod observer;
pub mod async_cacher;
mod websocket;

use std::{env, io};
use std::fs::File;
use std::path::PathBuf;
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};
use std::time::Duration;
use clap::Parser;
use tokio::{join, signal};
use crate::async_cacher::{AsyncCacher, SharedAsyncCacher};
use crate::cli::CLI;
use crate::observer::{Data, Observer};
use crate::server::Server;

static TIME_METRICS: [&str; 7] = ["nsec", "micsec", "msec", "sec", "min", "hour", "day"];

#[tokio::main]
async fn main() {
    let cli = CLI::parse();
    let path = cli.get_path();
    let host = cli.get_host();
    let with_logs = cli.with_logs();
    let with_autosave = cli.with_autosave();
    let autosave_delay = cli.autosave_delay();
    let connection_type = cli.get_connection_type();

    let (sender, mut receiver) = tokio::sync::mpsc::unbounded_channel();

    let sender_arc = Arc::new(sender);

    let data_putter =  SharedAsyncCacher.clone();
    let data_saver_auto = SharedAsyncCacher.clone();
    let data_saver_exit =  SharedAsyncCacher.clone();

    let running = Arc::new(AtomicBool::new(true));
    let r = running.clone();

    tokio::task::spawn( async move {
        signal::ctrl_c().await.expect("Failed to install Ctrl+C handler");
        r.store(false, Ordering::SeqCst);
        autosave(data_saver_exit).await;
        std::process::exit(0)
    });

    println!("Start Observing");

    if running.load(Ordering::SeqCst) {
        let server_task = tokio::task::spawn(async move {

            let server = Server::init(host, connection_type).await.unwrap();
            server.get_server().await.unwrap();
        });

        let observer_task = tokio::task::spawn(async move {
            let observer = Observer::init(&path).await;
            observer.run(sender_arc.clone(), with_logs)
        });

        let autosaver_task = tokio::task::spawn(async move {
            if with_autosave {
                let metric = autosave_delay.split(":").collect::<Vec<&str>>();
                let mut delay = match TIME_METRICS.iter().find(|&&x| x == metric[1]) {
                    Some(&"nsec") => tokio::time::interval(Duration::from_nanos(metric[0].parse::<u64>().unwrap())),
                    Some(&"micsec") => tokio::time::interval(Duration::from_micros(metric[0].parse::<u64>().unwrap())),
                    Some(&"msec") => tokio::time::interval(Duration::from_millis(metric[0].parse::<u64>().unwrap())),
                    Some(&"sec") => tokio::time::interval(Duration::from_secs(metric[0].parse::<u64>().unwrap())),
                    Some(&"min") => tokio::time::interval(Duration::from_secs(metric[0].parse::<u64>().unwrap() * 60)),
                    Some(&"hour") => tokio::time::interval(Duration::from_secs(metric[0].parse::<u64>().unwrap() * 60_u64.pow(2))),
                    Some(&"day") => tokio::time::interval(Duration::from_secs(metric[0].parse::<u64>().unwrap() * 60_u64.pow(2) * 24)),
                    None => panic!("Unknown time metric"),
                    Some(&&_) => panic!("Unknown time metric"),
                };

                loop {
                    delay.tick().await;
                    autosave(data_saver_auto.clone()).await;
                }
            }
        });

        while let Some(data) = receiver.recv().await {
            data_putter.put(data.0, data.1);
        }


        let _ = join!(server_task, observer_task, autosaver_task);

    }
}

async fn get_cache<'a>(cacher: Arc<AsyncCacher>) -> Vec<(PathBuf, Data)> {
    let mut vec = vec![];
    let data = cacher;
    for data in data.get().await {
        vec.push((data.0, data.1))
    }
    vec
}

async fn save_state(data: Vec<(PathBuf, Data)>) -> io::Result<()> {
    let path = env::current_exe().unwrap().parent().unwrap().join("state.json");
    let file = File::create(path)?;
    serde_json::to_writer(file, &data)?;
    Ok(())
}

async fn autosave(data_saver: Arc<AsyncCacher>) {
    let cache = get_cache(data_saver).await;
    if let Err(e) = save_state(cache).await {
        eprintln!("{}", e);
    } else {
        println!("State saved!");
    }
}
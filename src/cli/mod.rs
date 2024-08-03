use clap::Parser;
use crate::server::ConnectionType;

#[derive(Parser, Debug)]
#[command(version, about)]
#[warn(clippy::upper_case_acronyms)]
pub struct CLI {
    ///Observing path
    #[arg(short, long)]
    path: String,
    ///Print logs {action}: {filepath}
    #[arg(short, long)]
    logs: bool,
    ///Server address
    #[arg(long, default_value = "127.0.0.1:8080")]
    host: String,
    ///Auto save state to avoid critical failures
    #[arg(short, long)]
    autosave: bool,
    #[arg(short='d', long, default_value = "5:min")]
    autosave_delay: String,
    ///Connection type (w - Websocket, r - REST)
    #[arg(short,long)]
    connection_type: char
}

impl CLI {
    pub fn get_path(&self) -> String {
        self.path.clone()
    }
    pub fn get_host(&self) -> (String, u16) {
        let s = self.host.split(":").collect::<Vec<&str>>();
        (s[0].to_string(), s[1].parse::<u16>().unwrap())
    }
    pub fn with_logs(&self) -> bool {
        self.logs
    }
    pub fn with_autosave(&self) -> bool {
        self.autosave
    }
    pub fn autosave_delay(&self) -> String {
        self.autosave_delay.clone()
    }
    pub fn get_connection_type(&self) -> ConnectionType {
        match self.connection_type {
            'w' => ConnectionType::Websocket,
            'r' => ConnectionType::REST,
            _ => ConnectionType::Unknown
        }
    }
}

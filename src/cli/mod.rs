use clap::Parser;

#[derive(Parser, Debug)]
#[command(version, about)]
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
}
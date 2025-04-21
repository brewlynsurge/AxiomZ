use colored::{self, Colorize};
use url::Url;

/*
Info
*/
pub struct Info {
    name: String,
    version: String,
    part: String
}

impl Info {
    pub fn new(part: &str) -> Self {
        let version = env!("CARGO_PKG_VERSION");

        Self {
            name: String::from("SurfX"),
            version: String::from(version),
            part: String::from(part)
        }
    }

    pub fn display_head(&self) {
        println!("{} [{}]", self.part, self.version);
        println!("(c) {}. All rights are reserved.\n", self.name);
    }
}

/*
Console
*/
pub struct Console;
impl Console {
    pub fn info(msg: &str, module: Option<&str>) {
        print!("[INFO]  ");
        if module.is_some() {print!("[{}] ", module.unwrap().blue())}
        println!("{msg}")
    }

    pub fn warn(msg: &str, module: Option<&str>) {
        print!("[{}]  ", "WARN".yellow());
        if module.is_some() {print!("[{}] ", module.unwrap().blue())}
        println!("{msg}")
    }

    pub fn error(msg: &str, module: Option<&str>) {
        print!("[{}] ", "ERROR".red());
        if module.is_some() {print!("[{}] ", module.unwrap().blue())}
        println!("{}", msg.red())
    }
}

/*
Helper functions
*/
pub fn get_url_base(url: &str) -> Option<String> {
    let url = match Url::parse(url) {
        Ok(url) => url,
        Err(_) => return None,
    };

    if url.cannot_be_a_base() {return None;}

    let scheme = url.scheme();
    let host = match url.host_str() {
        Some(host) => host,
        None => return None,
    };

    Some(format!("{}://{}", scheme, host))
}
use colored::{self, Colorize};
use url::Url;

/* 
Global Variables
*/
pub const STOP_WORDS: [&str; 33]  = [
    "a", "an", "and", "are", "as", "at", "be", "but", "by", "for", "if", "in", "into", "is",
    "it", "no", "not", "of", "on", "or", "such", "that", "the", "their", "then", "there",
    "these", "they", "this", "to", "was", "will", "with",
];


pub static PROXY_FILE: &str = include_str!("../../http_proxies.txt");

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

// 
pub static URLS: [&str; 44] = [
    "https://www.google.com",
    "https://www.youtube.com",
    "https://www.facebook.com",
    "https://www.instagram.com",
    "https://chat.openai.com",
    "https://www.twitter.com",
    "https://www.whatsapp.com",
    "https://www.wikipedia.org",
    "https://www.reddit.com",
    "https://www.yahoo.co.jp",
    "https://www.yahoo.com",
    "https://www.yandex.com",
    "https://www.amazon.com",
    "https://www.baidu.com",
    "https://www.bet.br",
    "https://www.office.com",
    "https://www.linkedin.com",
    "https://www.netflix.com",
    "https://www.naver.com",
    "https://www.live.com",
    "https://www.zen.yandex.ru",
    "https://www.microsoft365.com",
    "https://www.bing.com",
    "https://www.temu.com",
    "https://www.pinterest.com",
    "https://www.bilibili.com",
    "https://www.microsoft.com",
    "https://www.twitch.tv",
    "https://www.vk.com",
    "https://www.mail.ru",
    "https://news.yahoo.co.jp",
    "https://www.sharepoint.com",
    "https://www.fandom.com",
    "https://www.globo.com",
    "https://www.canva.com",
    "https://www.weather.com",
    "https://www.samsung.com",
    "https://www.telegram.org",
    "https://www.duckduckgo.com",
    "https://www.openai.com",
    "https://www.nytimes.com",
    "https://www.zoom.us",
    "https://www.aliexpress.com",
    "https://www.roblox.com",
];

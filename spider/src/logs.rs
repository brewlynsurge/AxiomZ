use colored::{self, Colorize};

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
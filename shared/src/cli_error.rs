use std::process::exit;
use crossterm::style::Stylize;

// ------------------- ERROR ENTITY -----------------------
struct ErrorEntity {
    pub kind: String,
    pub msg: String
}

// -------------------- CLI ERROR -------------------------
pub struct CliError {
    errors: Vec<ErrorEntity>,
    warnings: Vec<String>
}

impl CliError {
    pub fn new() -> Self {
        Self {
            errors: Vec::new(),
            warnings: Vec::new()
        }
    }

    pub fn error(mut self, kind: &str, msg: &str) -> Self {
        self.push_error(kind, msg);
        self
    }

    pub fn push_error(&mut self, kind: &str, msg: &str) {
        self.errors.push(ErrorEntity {
            kind: kind.to_string(),
            msg: msg.to_string()
        });
    }

    pub fn warning(mut self, msg:&str) -> Self {
        self.warnings.push(msg.to_string());
        self
    }

    pub fn report(&self) {
        // Error Messages
        let mut error_msg: String = String::from(&format!("{}", "[ERROR]".red()));

        for cli_error in self.errors.iter() {
            error_msg.push_str(&format!("\n   {} {}", "-".red(), cli_error.kind.clone().red()));
            error_msg.push_str(&format!("{} {}", ":".red(), cli_error.msg.clone()));
        }
        error_msg.push_str("\n");
    
        // Warnings
        let mut warn_msg = String::new();
        for cli_warn_msg in self.warnings.iter() {
            warn_msg.push_str(&format!("{} {}", "warning:".yellow(), cli_warn_msg));
            warn_msg.push_str("\n");
        }

        // Report errors and warnings
        if !self.errors.is_empty() { println!("{error_msg}") } 
        if !self.warnings.is_empty() { println!("{warn_msg}") }

        if !self.errors.is_empty() {
            exit(1);
        }
    }
}

// ---------------- raise_error Macro ---------------------
#[macro_export]
macro_rules! raise_error {
    ($kind:ident, $($msg:tt)*) => {{
        let error_msg = format!($($msg)*);

        CliError::new()
            .error(stringify!($kind), &error_msg)
            .report();
        std::process::exit(1);
    }};

    ($kind:literal, $($msg:tt)*) => {{
        let error_msg = format!($($msg)*);

        CliError::new()
            .error($kind, &error_msg)
            .report();
        std::process::exit(1);
    }};
}
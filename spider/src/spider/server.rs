use tokio::net::TcpListener;
use crossterm::{cursor, style::Stylize};
use std::io::{Write, Stdout};
use std::process::exit;
use std::thread;

use spider_shared::database::AxiomZDatabase;
use crate::terminal;

// ----------------- SERVER GLOBALS ----------------------
const RESET_DATABASE: bool = true;

// ----------------- AXIOMZ SERVER ----------------------
pub struct AxiomZServer {
    _cursor_guard: CursorGuard,
    configurations: Option<Configurations>,
    server_core: Option<ServerCore>,
    database: Option<AxiomZDatabase>
}

impl AxiomZServer {
    pub fn new() -> AxiomZServer {
        let cursor_guard = CursorGuard::new().unwrap();
        
        Self {
            _cursor_guard: cursor_guard,
            configurations: None,
            server_core: None,
            database: None
        }
    }

    pub async fn start(&mut self) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
        ServerInitiator::initialize(self).await?;
        let configurations = self.configurations.as_ref().unwrap();
        
        // Server Task
        let server_task = {
            let server_core = self.server_core.take().unwrap();
            let database_config = configurations.database.clone();
            
            tokio::spawn(async move {
                server_core.handle_connections(&database_config).await;
            })
        };

        // AxiomZ Terminal Task

        
        // Joining Task
        _ = tokio::join!(server_task); // TODO

        
        Ok(())
    }
}

// ----------------- SERVER CORE ----------------------
struct ServerCore {
    tcp_listener: TcpListener
}

impl ServerCore {
    pub async fn build(spider_config: &shared::config::SpiderConfig) -> Result<Self, Box<dyn std::error::Error + Send + Sync>> {
        let tcp_listener = TcpListener::bind(format!("{}:{}", spider_config.host, spider_config.port)).await?;
        
        Ok(Self {
            tcp_listener: tcp_listener
        })
    }

    pub async fn handle_connections(&self, database_config: &shared::config::DatabaseConfig) {
        tokio::time::sleep(tokio::time::Duration::from_secs(3)).await;
        loop {
            
        }
    }
}

// ----------------- SERVER HELPERS ----------------------
struct Configurations {
    pub database: shared::config::DatabaseConfig,
    pub spider: shared::config::SpiderConfig
}

// ----------------- SERVER INITIATOR ----------------------
struct ServerInitiator;
impl ServerInitiator {
    pub async fn initialize(server: &mut AxiomZServer) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
        let mut stdout = std::io::stdout();
        let mut lines_written:u16 = 0;

        writeln!(stdout, "{}", "AxiomZ Server".bold())?;
        write!(stdout, " {} {}\n", "[STATE]:".cyan(), "initializing".italic().green())?;
        lines_written += 2;

        // Initiaizing
        Self::init_configurations(server, &mut stdout, &mut lines_written)?;
        Self::init_tcp_listener(server, &mut stdout, &mut lines_written).await?;
        Self::init_database(server, &mut stdout, &mut lines_written).await?;
        
        Ok(())
    }

    fn init_configurations(server: &mut AxiomZServer, stdout: &mut Stdout, lines_written: &mut u16) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
        write!(stdout, "    {} loading configurations: ", "-".bold().green())?;
        let config_loader = shared::config::ConfigLoader::new()
            .resolve("DATABASE")
            .resolve("SPIDER")
            .execute_resolves();

        let database_config = shared::config::DatabaseConfig::load(&config_loader);
        let spider_config = shared::config::SpiderConfig::load(&config_loader);
        server.configurations = Some(Configurations {
            database: database_config,
            spider: spider_config
        });
        writeln!(stdout, "{}", "done".bold().green())?;
        *lines_written += 1;
        
        Ok(())
    }

    async fn init_tcp_listener(server: &mut AxiomZServer, stdout: &mut Stdout, lines_written: &mut u16) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
        crossterm::queue!(stdout, cursor::SavePosition)?;
        write!(stdout, "    {} initializing tcp listener: ", "-".bold().green())?;
        stdout.flush()?;

        let configurations = server.configurations.as_ref().unwrap();
        let addr = format!("{}:{}", configurations.spider.host, configurations.spider.port);
        
        match ServerCore::build(&configurations.spider).await {
            Ok(s) => {server.server_core = Some(s)}
            Err(e) => {
                writeln!(stdout, "{}", "failed".bold().red())?;
                writeln!(stdout, "       {} {}", ">".red().bold(), e.to_string().red().italic())?;
                exit(1);
            }
        }
        
        write!(stdout, "{}", "done".bold().green())?;
        crossterm::queue!(stdout, cursor::RestorePosition, crossterm::terminal::Clear(crossterm::terminal::ClearType::CurrentLine))?;
        writeln!(stdout, "    {} server listening: {}", "-".bold().green(), addr.italic().cyan())?;
        *lines_written += 1;

        Ok(())
    }

    async fn init_database(server: &mut AxiomZServer, stdout: &mut Stdout, lines_written: &mut u16) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
        write!(stdout, "    {} starting database: ", "-".bold().green())?;
        stdout.flush()?;
        
        let database_config = {
            let configs = server.configurations.as_ref().unwrap();
            &configs.database
        };
        match AxiomZDatabase::connect(database_config).await {
            Ok(axiomz_database) => {
                if RESET_DATABASE {
                    write!(stdout, "\n      {} reseting database: ", ">".bold().green())?;
                    axiomz_database.reset_database().await?;
                    stdout.flush()?;
                    thread::sleep(std::time::Duration::from_millis(500));
                }
                
                axiomz_database.run_migrations().await?;
                server.database = Some(axiomz_database)
            },
            Err(e) => {
                writeln!(stdout, "{}", "failed".bold().red())?;
                writeln!(stdout, "       {} {}", ">".red().bold(), e.to_string().red().italic())?;
                exit(1);
            }
        }
        writeln!(stdout, "{}", "done".bold().green())?;
        *lines_written += 1;
        if RESET_DATABASE {
            *lines_written += 1;
            thread::sleep(std::time::Duration::from_millis(1000));
        }
        
        Ok(())
    }
}

// ----------------- CURSOR GUARD ----------------------
struct CursorGuard;
impl CursorGuard {
    pub fn new() -> Result<Self, Box<dyn std::error::Error + Send + Sync>> {
        crossterm::execute!(std::io::stdout(), cursor::Hide)?;
        Ok(Self {})
    }
}

impl Drop for CursorGuard {
    fn drop(&mut self) {
        let _ = crossterm::execute!(std::io::stdout(), cursor::Show);
    }
}
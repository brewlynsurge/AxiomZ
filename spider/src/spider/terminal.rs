use std::collections::{VecDeque};
use tokio::sync::{Mutex, Notify, OnceCell};
use tokio::task::JoinHandle;
use std::io::Write;
use crossterm::{
    cursor, execute, queue,
    style::{Print, Stylize},
    terminal::{Clear, ClearType},
};

// ------------------- AXIOMZ TERMINAL -----------------------
pub static AXIOMZ_TERMINAL: OnceCell<TerminalHandler> = OnceCell::const_new();

// ------------------- TERMINAL HANDLER ------------------------
pub struct TerminalHandler {
    msg_queue: Mutex<VecDeque<String>>,
    redraw_notify: Notify,
    terminal_mode: Mutex<TerminalMode>
}

impl TerminalHandler {
    pub fn new() -> Self {
        TerminalHandler {
            msg_queue: Mutex::new(VecDeque::new()),
            redraw_notify: Notify::new(),
            terminal_mode: Mutex::new(TerminalMode::Passive)
        }
    }

    pub async fn push_msg(&self, msg: &str) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
        let terminal_mode = self.terminal_mode.lock().await;
        if matches!(*terminal_mode, TerminalMode::Passive) {
            let mut msg_queue = self.msg_queue.lock().await;
            msg_queue.push_back(msg.into());
        } else {

            //if self.terminal_messages.len() as u16 >= 5 {
            //    self.terminal_messages.pop_front();
            //}

            //self.terminal_messages.push_back(msg.into());
            todo!()
        }


        Ok(())
    }
}

// ------------------- TERMINAL MODE SYSTEM ------------------------
#[derive(Debug, Clone, Copy)]
enum TerminalMode {
    Passive,
    Dashboard
}

struct TerminalModeSystem {
    pub mode: TerminalMode,
    dashboard_task_handle: Option<JoinHandle<()>>  // TODO: Change string to joinhandle
}

impl TerminalModeSystem {
    pub fn new() -> Self {
        let mut mode_system = Self {
            mode: TerminalMode::Passive,
            dashboard_task_handle: None
        };
        
        if let Err(e) = mode_system.to_passive_mode() {
            panic!("Failed to change to passive mode at the initialization of the TerminalModeSystem: {}", e)
        }
        mode_system
    }
    
    pub fn to_passive_mode(&mut self) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
        if let Some(dashboard_task_handle) = self.dashboard_task_handle.as_ref() {
            
        }

        self.mode = TerminalMode::Passive;

        Ok(()) // TODO: Verify if it need to return Result
    }

    pub fn to_dashboard_mode(&mut self) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
        if self.dashboard_task_handle.is_some() {
            panic!("Already runnning axiomz terminal in DashBoard Mode")
        }

        let task_handle = tokio::spawn(async move {
            todo!()
        });

        self.dashboard_task_handle = Some(task_handle);
        self.mode = TerminalMode::Dashboard;
        Ok(())
    }
}

// ------------------- MACRO FUNCTIONS ---------------------
macro_rules! log {
    ($log_mode:ident -> $($arg:tt)*) => {{
        // Initialize AxiomZTerminal (if not initialized yet)
        let axiomz_terminal = AXIOMZ_TERMINAL.get_or_init(|| async {
            // Spawing rendering task
            tokio::spawn(async {
                let mut terminal_renderer = TerminalRenderer::new();
                if let Err(e) = terminal_renderer.handle_rendering().await {
                    panic!("The Terminal renderer failed: {e}")
                }
            });
            println!("{}", TerminalRenderer::get_head());

            TerminalHandler::new()
        }).await;

        match stringify!($log_mode) {
            "OVERWRITE" => {
                // Go to previous line and clear it
                let mut stdout = std::io::stdout();
                let _ = crossterm::execute!(
                    stdout,
                    crossterm::cursor::MoveUp(1),
                    crossterm::cursor::MoveToColumn(0),
                    crossterm::terminal::Clear(crossterm::terminal::ClearType::CurrentLine)
                );
            },
            _ => {}
        }

        if let Err(e) = axiomz_terminal.push_msg(&format!($($arg)*)).await {
            panic!("AxiomZTerminal log! failed: {e}")
        }


    }};

    ($($arg:tt)*) => {{
        // Initialize AxiomZTerminal (if not initialized yet)
        let axiomz_terminal = AXIOMZ_TERMINAL.get_or_init(|| async {
            // Spawing rendering task
            tokio::spawn(async {
                let mut terminal_renderer = TerminalRenderer::new();
                if let Err(e) = terminal_renderer.handle_rendering().await {
                    panic!("The Terminal renderer failed: {e}")
                }
            });
            println!("{}", TerminalRenderer::get_head());

            TerminalHandler::new()
        }).await;

        if let Err(e) = axiomz_terminal.push_msg(&format!($($arg)*)).await {
            panic!("AxiomZTerminal log! failed: {e}")
        }
    }};


}
pub(crate) use log;


// ------------------ TERMINAL RENDERER ----------------------
pub struct TerminalRenderer;
impl TerminalRenderer {
    pub fn new() -> Self {
        Self {}
    }

    pub fn get_head() -> String {
        let mut head = format!("{} [Version {}]\n", "Spider".cyan().bold(), env!("CARGO_PKG_VERSION").italic());
        head.push_str(&format!("(c) {}. All rights are reserved.\n", "AxiomZ".cyan().italic()));
        head
    }

    async fn get_terminal_mode(axiomz_ter: &TerminalHandler) -> TerminalMode {
        let mode = axiomz_ter.terminal_mode.lock().await;
        *mode
    }

    pub async fn handle_rendering(&mut self) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
        // Wait for AXIOMZ_TERMINAL to be initialized
        let axiomz_terminal = loop {
            if let Some(axiomz_terminal) = AXIOMZ_TERMINAL.get() {
                break axiomz_terminal
            }
            else { tokio::time::sleep(tokio::time::Duration::from_millis(200)).await }
        };

        // First handle passive rendering
        while matches!(Self::get_terminal_mode(&axiomz_terminal).await, TerminalMode::Passive) {
            let mut msg_queue = axiomz_terminal.msg_queue.lock().await;

            if let Some(msg) = msg_queue.pop_front() {
                println!("{}", msg)
            }

            drop(msg_queue);
            tokio::time::sleep(tokio::time::Duration::from_millis(100)).await;
        }

        // Handle Dashboard rendering
        todo!();

        Ok(())
    }

    pub async fn draw() {
        todo!()
    }
}


// ----------------- AlternateScreenGuard ---------------------
pub struct AlternateScreenGuard;
impl AlternateScreenGuard {
    pub fn new() -> Result<Self, Box<dyn std::error::Error + Send + Sync>> {
        Self::enter_alternate_state()?;
        Ok(Self {})
    }

    fn enter_alternate_state() -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
        let mut out = std::io::stdout();

        crossterm::terminal::enable_raw_mode()?;
        queue!(out, crossterm::terminal::EnterAlternateScreen)?;
        queue!(out, Clear(ClearType::All))?;
        queue!(out, cursor::MoveTo(0, 0))?;

        // Ensure cleanup runs even on panic, before the panic message prints
        let default_hook = std::panic::take_hook();
        std::panic::set_hook(Box::new(move |info| {
            let _ = crossterm::terminal::disable_raw_mode();
            let _ = execute!(std::io::stdout(), crossterm::terminal::LeaveAlternateScreen);
            default_hook(info);
        }));

        out.flush()?;
        Ok(())
    }
}

impl Drop for AlternateScreenGuard {
    fn drop(&mut self) {
        let _ = crossterm::terminal::disable_raw_mode();
        let _ = execute!(std::io::stdout(), crossterm::terminal::LeaveAlternateScreen);
    }
}

/*
use crossterm::{
    cursor, execute, queue,
    style::{Print, Stylize},
    terminal::{Clear, ClearType},
};
use shared::spider_api;
use std::collections::VecDeque;
use std::sync::atomic::{AtomicU32, Ordering};
use std::{
    io::Write,
    sync::{Arc, LazyLock},
};
use tokio::sync::{Mutex, mpsc};

// ----------------- DECLARING TERMINAL -------------------
pub static TERMINAL: LazyLock<Mutex<AxiomZTerminal>> =
    LazyLock::new(|| Mutex::new(AxiomZTerminal::new()));

// ----------------- AXIOMZ TERMINAL ----------------------
pub struct AxiomZTerminal {
    pub terminal_tx: mpsc::Sender<String>,
    terminal_rx: Option<mpsc::Receiver<String>>,
    terminal_messages: VecDeque<String>,

    _alternate_mode_guard: AternateModeGuard,
    start_row: u16,

    speed_display: Arc<Mutex<Option<String>>>,
}

impl AxiomZTerminal {
    pub fn new() -> Self {
        let (tx, rx) = mpsc::channel::<String>(5);

        Self {
            terminal_tx: tx,
            terminal_rx: Some(rx),
            terminal_messages: VecDeque::new(),
            _alternate_mode_guard: AternateModeGuard {},
            start_row: 0,
            speed_display: Arc::new(Mutex::new(None)),
        }
    }

    pub async fn start(&mut self) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
        let mut out = std::io::stdout();
        AternateModeGuard::enter_alternate_mode(&mut out)?;

        execute!(out, Print("\n"))?;
        (_, self.start_row) = cursor::position()?;

        // Reserving space for the terminal
        for _ in 0..11 {
            println!();
        }

        // Spawning Speed Instrument Calculation Task
        let speed_display = self.speed_display.clone();
        tokio::spawn(async move {
            match CrawlerSpeedInstrument::handle_speed_calculation(speed_display).await {
                Ok(_) => {}
                Err(e) => {
                    println!("CrawlerSpeedInstrument failed: {e}")
                }
            }
        });

        self.start_update_loop().await?;
        Ok(())
    }

    async fn start_update_loop(&mut self) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
        let mut terminal_rx = self.terminal_rx.take().unwrap();
        self.draw().await?;

        loop {
            tokio::select! {
                _ = tokio::time::sleep(tokio::time::Duration::from_secs(5)) => {
                    self.draw().await?;
                }
                recv_msg = terminal_rx.recv() => {
                    match recv_msg {
                        Some(recv_msg) => {
                            self.push_msg(&recv_msg)?;
                        }
                        None => { println!("Channel closed") }
                    }
                }
            }
        }
    }

    pub fn push_msg(&mut self, msg: &str) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
        if self.terminal_messages.len() as u16 >= 5 {
            self.terminal_messages.pop_front();
        }

        self.terminal_messages.push_back(msg.into());
        Ok(())
    }

    async fn draw(&mut self) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
        let mut out = std::io::stdout();
        queue!(out, cursor::MoveTo(0, 0))?;

        // Drawing speed
        queue!(
            out,
            cursor::MoveTo(0, self.start_row),
            Clear(ClearType::CurrentLine)
        )?;
        {
            let speed_display = self.speed_display.lock().await;
            if let Some(speed_display) = speed_display.as_ref() {
                queue!(
                    out,
                    Print(format!(
                        " {} {}: {}",
                        "-".green().bold(),
                        "Speed".italic(),
                        speed_display.clone().italic()
                    ))
                )?;
            } else {
                queue!(
                    out,
                    Print(format!(
                        " {} {}: calculating...",
                        "-".green().bold(),
                        "Speed".italic()
                    ))
                )?;
            }
        }

        // No of workers
        {
            let no_of_crawlers = spider_api::CRAWLER_GLOBALS
                .active_crawler_count
                .lock()
                .await;
            queue!(
                out,
                cursor::MoveTo(0, self.start_row + 1),
                Clear(ClearType::CurrentLine)
            )?;
            queue!(
                out,
                Print(format!(
                    " {} {}: {}",
                    "-".green().bold(),
                    "Crawlers".italic(),
                    no_of_crawlers
                ))
            )?;
        }

        // Total Links Crawled: TODO
        queue!(
            out,
            cursor::MoveTo(0, self.start_row + 3),
            Clear(ClearType::CurrentLine)
        )?;
        queue!(out, Print(format!(" {}: TODO", "Total Links Crawled")))?;

        // Logs
        queue!(
            out,
            cursor::MoveTo(0, self.start_row + 5),
            Clear(ClearType::CurrentLine)
        )?;
        queue!(
            out,
            Print(format!("{}", "------ Terminal Logs ------".bold()))
        )?;

        for i in 0..6 {
            let pos: u16 = self.start_row + 6 + i as u16;
            queue!(out, cursor::MoveTo(0, pos), Clear(ClearType::CurrentLine))?;

            if let Some(msg) = self.terminal_messages.get(i) {
                queue!(out, Print(msg))?;
            }
        }

        out.flush()?;
        Ok(())
    }
}

// ----------------- TERMINAL ALTERNATE MODE GUARD ----------------------
struct AternateModeGuard;
impl AternateModeGuard {
    fn get_head() -> String {
        let mut head = format!(
            "{} [Version {}]\n",
            "Spider".cyan().bold(),
            env!("CARGO_PKG_VERSION").italic()
        );
        head.push_str(&format!(
            "(c) {}. All rights are reserved.\n",
            "AxiomZ".cyan().italic()
        ));

        return head;
    }

    pub fn enter_alternate_mode(
        out: &mut std::io::Stdout,
    ) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
        crossterm::terminal::enable_raw_mode()?;
        queue!(out, crossterm::terminal::EnterAlternateScreen)?;
        queue!(out, Clear(ClearType::All))?;
        queue!(out, cursor::MoveTo(0, 0))?;

        // print head
        queue!(out, Print(Self::get_head()))?;
        out.flush()?;

        // Ensure cleanup runs even on panic, before the panic message prints
        let default_hook = std::panic::take_hook();
        std::panic::set_hook(Box::new(move |info| {
            let _ = crossterm::terminal::disable_raw_mode();
            let _ = execute!(std::io::stdout(), crossterm::terminal::LeaveAlternateScreen);
            default_hook(info);
        }));

        Ok(())
    }
}

impl Drop for AternateModeGuard {
    fn drop(&mut self) {
        let _ = crossterm::terminal::disable_raw_mode();
        let _ = execute!(std::io::stdout(), crossterm::terminal::LeaveAlternateScreen);
    }
}

// ----------------- CRAWLER SPEED INSTRUMENT ----------------------
struct CrawlerSpeedInstrument {
    crawl_count: AtomicU32,
}

pub static CRAWLER_SPEED_INSTRUMENT: LazyLock<CrawlerSpeedInstrument> =
    LazyLock::new(|| CrawlerSpeedInstrument {
        crawl_count: AtomicU32::new(0),
    });

impl CrawlerSpeedInstrument {
    pub fn increment_crawl(&self) {
        self.crawl_count.fetch_add(1, Ordering::Relaxed);
    }

    pub async fn handle_speed_calculation(
        speed_display: Arc<Mutex<Option<String>>>,
    ) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
        let mut min_interval = tokio::time::interval(tokio::time::Duration::from_secs(60));

        tokio::time::sleep(tokio::time::Duration::from_secs(60)).await;
        loop {
            min_interval.tick().await;

            let crawl_count = CRAWLER_SPEED_INSTRUMENT
                .crawl_count
                .swap(0, Ordering::Relaxed);
            let mut s_display = speed_display.lock().await;
            *s_display = Some(format!("{} {}", crawl_count, "crawls/min"));
        }
    }
}

// ----------------- MACRO FUNCTIONS ----------------------
macro_rules! console_log {
    ($($arg:tt)*) => {{
        let msg = format!($($arg)*);
        let s_terminal = TERMINAL.lock().await;
        let _ = s_terminal.terminal_tx.send(msg).await;
    }};
}
pub(crate) use console_log;
/*

 - Speed: calculating...    |       Speed: 5 crawls/min
 - No of Workers: 3

 Total Links Crawled: 2000 pages

 ------ Logs -----
1)
2)
3)
4)
5)
*/

// Reexport macro
// pub(crate) use macro
*/

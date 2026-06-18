use tokio::sync::{mpsc, Mutex};
use std::sync::Arc;
use std::io::Write;
use crossterm::{style::Stylize, cursor, terminal};
use url::Url;

use crate::crawler_animator::DrawProcessingMode::{Processing, Saving};


// ----------------- CRAWLER ANIMATOR ----------------------
pub struct CrawlerAnimator {
    stdout: Arc<Mutex<std::io::Stdout>>,
    processing_tx: mpsc::Sender<DrawProcessingCommands>,
    processing_rx: Option<mpsc::Receiver<DrawProcessingCommands>>
}

impl CrawlerAnimator {
    pub fn new() -> Self {
        let stdout = std::io::stdout();
        let (tx, rx) = mpsc::channel::<DrawProcessingCommands>(10);
        Self {
            stdout: Arc::new(Mutex::new(stdout)),
            processing_tx: tx,
            processing_rx: Some(rx)
        }
    }

    pub async fn initialize(&mut self) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
        // Spawn processing animation task
        let entry_parameters = DrawProcessingParameters {
            stdout: self.stdout.clone(),
            rx: self.processing_rx.take().unwrap()
        };
        
        tokio::spawn(async move {
            match AnimatorInstance::draw_processing_animation(entry_parameters).await {
                Ok(_) => {},
                Err(e) => {panic!("{}", e)}
            }
        });
        self.print_head().await?;

        Ok(())
    }

    async fn print_head(&self) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
        let mut stdout = self.stdout.lock().await;
        
        writeln!(stdout, "\n{} {} {}", "▶".cyan().bold(), "CRAWLER".bold(), "◀".cyan().bold())?;
        stdout.flush()?;
        Ok(())
    }

    pub async fn create_instance(&mut self, page_url: &str) -> Result<AnimatorInstance, Box<dyn std::error::Error + Send + Sync>> {
        let tx = self.processing_tx.clone();
        tx.send(DrawProcessingCommands::SendUrl(page_url.to_string())).await?;

        Ok(AnimatorInstance {
            processing_tx: tx
        })
    }
}

// ----------------- ANIMATOR HELPERS ----------------------
enum DrawProcessingCommands {
    SendUrl(String),
    StartProcessing,
    ToSaving,
    Done(DrawProcessingResult)
}

enum DrawProcessingMode {
    Processing,
    Saving
}

impl DrawProcessingMode {
    pub fn to_string(&self) -> String {
        match self {
            Processing => String::from("Processing"),
            Saving => String::from("Saving")
        }
    }
    
    pub fn apply_style(&self) -> crossterm::style::StyledContent<String> {
        match self {
            Processing => String::from("Processing").blue(),
            Saving => String::from("Saving").green()
        }
    } 
}

enum DrawProcessingResult {
    Success,
    Failure(Option<String>)
}

struct DrawProcessingParameters {
    stdout: Arc<Mutex<std::io::Stdout>>,
    rx: mpsc::Receiver<DrawProcessingCommands>
}

// ----------------- ANIMATOR INSTANCE ----------------------
pub struct AnimatorInstance {
    processing_tx: mpsc::Sender<DrawProcessingCommands>
}

impl AnimatorInstance {
    const PRETTIFY_MAX_LEN: usize = 50;
    pub async fn to_processing(&self) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
        self.processing_tx.send(DrawProcessingCommands::StartProcessing).await?;
        Ok(())
    }

    pub async fn to_saving(&self) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
        self.processing_tx.send(DrawProcessingCommands::ToSaving).await?;
        Ok(())
    }

    pub async fn to_success(&self) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
        self.processing_tx.send(DrawProcessingCommands::Done(DrawProcessingResult::Success)).await?;
        Ok(())
    }

    pub async fn to_failure(&self, error_msg: Option<String>) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
        self.processing_tx.send(DrawProcessingCommands::Done(DrawProcessingResult::Failure(error_msg))).await?;
        Ok(())
    }
    
    async fn draw_processing_animation(mut entry_parameters: DrawProcessingParameters) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
        let mut first_time = true;
        loop {
            let page_url = {
                match entry_parameters.rx.recv().await {
                    Some(DrawProcessingCommands::SendUrl(u)) => u,
                    _ => {panic!("Expected a page url")}
                }
            };

            {
                let mut stdout = entry_parameters.stdout.lock().await;
                if !first_time { writeln!(stdout, "")?}
                let first_layer_msg = format!("{} 🔗 {}", "Crawling".bold().cyan(), Self::prettify_url(&page_url, Self::PRETTIFY_MAX_LEN));
                writeln!(stdout, "{}", first_layer_msg)?;
                stdout.flush()?;
            }

            match entry_parameters.rx.recv().await {
                Some(DrawProcessingCommands::StartProcessing) => {},
                _ => {panic!("Expected a to_processing command")}
            }
            
            let mut stdout = entry_parameters.stdout.lock().await;
            let mut processing_mode = DrawProcessingMode::Processing;
            let frames = ['|', '/', '-', '\\'];
            let (_, animation_row) = cursor::position()?;
            
            'animation_loop: loop {
                for f in frames.iter() {
                    tokio::select! {
                        _ = tokio::time::sleep(tokio::time::Duration::from_millis(360)) => {
                            crossterm::queue!(
                                stdout,
                                cursor::MoveTo(0, animation_row),
                                terminal::Clear(terminal::ClearType::CurrentLine)
                            )?;

                            let second_layer_msg = format!("   - {} {}", processing_mode.apply_style(), f);
                            crossterm::queue!(stdout, cursor::MoveTo(0, animation_row))?;
                            write!(stdout, "{}", second_layer_msg)?;
                            stdout.flush()?;
                        }
                        command = entry_parameters.rx.recv() => {
                            match command {
                                Some(DrawProcessingCommands::ToSaving) => {
                                    processing_mode = DrawProcessingMode::Saving
                                },
                                Some(DrawProcessingCommands::Done(p_result)) => {
                                    match p_result {
                                        DrawProcessingResult::Success => {
                                            AnimatorInstance::play_success(&mut stdout, &page_url, animation_row, first_time).await?;
                                            break 'animation_loop;
                                        },
                                        DrawProcessingResult::Failure(error_msg) => {
                                            AnimatorInstance::play_failure(&mut stdout, &page_url, animation_row, &processing_mode, error_msg, first_time).await?;
                                            break 'animation_loop;
                                        }
                                    }
                                },
                                _ => {panic!("Unexpected command during animation processing!")}
                            }
                        }
                    }
                }
            }
            first_time = false;
        }

        

        
    }

    async fn play_success(stdout: &mut tokio::sync::MutexGuard<'_, std::io::Stdout>, url: &str, animation_row: u16, first_time: bool) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
        crossterm::queue!(
            stdout,
            cursor::MoveTo(0, animation_row),
            terminal::Clear(terminal::ClearType::CurrentLine),
            cursor::MoveTo(0, animation_row-1),
            terminal::Clear(terminal::ClearType::CurrentLine)
        )?;

        if !first_time {
            crossterm::queue!(stdout, cursor::MoveTo(0, animation_row-2), terminal::Clear(terminal::ClearType::CurrentLine))?;
        }
        
        writeln!(stdout, "{} 🔗 {}", "Success".green().bold(), Self::prettify_url(url, Self::PRETTIFY_MAX_LEN))?;
        stdout.flush()?;
        Ok(())
    }

    async fn play_failure(stdout: &mut tokio::sync::MutexGuard<'_, std::io::Stdout>, url: &str, animation_row: u16, processing_mode: &DrawProcessingMode, error_msg: Option<String>, first_time: bool) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
        crossterm::queue!(
            stdout,
            cursor::MoveTo(0, animation_row),
            terminal::Clear(terminal::ClearType::CurrentLine),
        )?;

        let new_second_layer_msg = format!("   - {}: failed", processing_mode.to_string());
        write!(stdout, "{}", new_second_layer_msg.red())?;
        stdout.flush()?;
        tokio::time::sleep(tokio::time::Duration::from_millis(500)).await;

        crossterm::queue!(
            stdout,
            cursor::MoveTo(0, animation_row),
            terminal::Clear(terminal::ClearType::CurrentLine),
            cursor::MoveTo(0, animation_row-1),
            terminal::Clear(terminal::ClearType::CurrentLine)
        )?;

        if !first_time {
            crossterm::queue!(stdout, cursor::MoveTo(0, animation_row-2), terminal::Clear(terminal::ClearType::CurrentLine))?;
        }

        write!(stdout, "{}  🔗 {}\n", "Failed".red().bold(), Self::prettify_url(url, Self::PRETTIFY_MAX_LEN))?;
        if let Some(error_msg) = error_msg {
            write!(stdout, "   {} {}\n", ">".cyan(), error_msg.red().italic())?;
        }
        stdout.flush()?;
        
        Ok(())
    }

    fn prettify_url(url: &str, max_len: usize) -> String {
        let Ok(parsed) = Url::parse(url) else {return url.chars().take(max_len).collect();};

        let host = parsed.host_str().unwrap_or("?");
        let base = format!("{}://{}", parsed.scheme(), host);
        let segments: Vec<&str> = parsed
            .path_segments()
            .map(|s| s.filter(|s| !s.is_empty()).collect())
            .unwrap_or_default();

        let fit = |s: &str| s.chars().count() <= max_len;
        let trim = |s: &str| s.chars().take(max_len).collect::<String>();
        let full = match segments.as_slice() {
            [] => base.clone(),
            [only] => format!("{}/{}", base, only),
            _ => format!("{}/{}", base, segments.join("/")),
        };

        if fit(&full) {return full;}
        let last = segments.last().unwrap();
        let collapsed = format!("{}/../{}", base, last);
        if !fit(&collapsed) {return trim(&collapsed);}

        // Use saturating_sub to avoid usize underflow
        let fixed_cost = base.chars().count()
            + 1               // '/' after base
            + "/../".chars().count()
            + last.chars().count();

        let budget = max_len.saturating_sub(fixed_cost);
        if budget == 0 {return collapsed;}

        let mut used = 0;
        let leading: Vec<&str> = segments
            .iter()
            .take(segments.len() - 1)
            .take_while(|seg| {
                let cost = seg.chars().count() + 1;
                used += cost;
                used <= budget
            }).copied().collect();

        match leading.as_slice() {[] => collapsed,
            _ => format!("{}/{}/../{}", base, leading.join("/"), last),
        }
    }
}
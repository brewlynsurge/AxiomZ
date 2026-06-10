use tokio::sync::mpsc;
use crossterm::{style::Stylize, cursor, terminal};
use std::io::Write;
use url::Url;


// ----------------- ANIMATOR STATES ----------------------
pub struct InitialState;
pub struct ProcessingState;
pub struct FinalState;

// ----------------- ANIMATOR ----------------------
pub struct Animator<State=InitialState> {
    stdout: Option<std::io::Stdout>,
    state: std::marker::PhantomData<State>,
    processing_tx: Option<mpsc::Sender<ProcessingChannelCommand>>,
    processing_hander: Option<tokio::task::JoinHandle<Result<ProcessingTaskReturns, Box<dyn std::error::Error + Send + Sync>>>>
}

impl Animator {
    pub fn new() -> Self {
        let terminal_out = std::io::stdout();
        
        Self {
            stdout: Some(terminal_out),
            state: Default::default(),
            processing_tx: None,
            processing_hander: None
        }
    }
}

// ----------------- ANIMATOR - InitialState Implementation ----------------------
impl Animator<InitialState> {
    pub fn print_head(&mut self) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
        let out = self.stdout.as_mut().ok_or("'stdout' field has been moved from CrawlerAnimator")?;

        let head_msg = format!("\n{} {} {}\n", "--".bold().cyan(), "CRAWLER".bold(), "--".bold().cyan());
        write!(out, "{}", head_msg)?;
        out.flush()?;
        Ok(())
    }

    pub async fn process(&mut self, url: &str) -> Result<Animator<ProcessingState>, Box<dyn std::error::Error + Send + Sync>> {
        let terminal_out = self.stdout.take().unwrap();
        let mut animator = Animator {
            stdout: Some(terminal_out),
            state: std::marker::PhantomData::<ProcessingState>,
            processing_tx: None,
            processing_hander: None
        };

        animator.start(url).await?;
        
        return Ok(animator)
    }
}

// ----------------- ANIMATOR - ProcessingState Implementation ----------------------
#[derive(Debug, Clone)]
enum ProcessingChannelCommand {
    ToSaving,
    Done
}

struct ProcessingTaskParameters {
    pub url: String,
    pub stdout: Option<std::io::Stdout>,
    pub rx: mpsc::Receiver<ProcessingChannelCommand>
}

struct ProcessingTaskReturns {
    pub stdout: Option<std::io::Stdout>,
    first_layer_msg: String,
    second_layer_msg: String,
    animation_row: u16
}

impl Animator<ProcessingState> {
    async fn start(&mut self, url: &str) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
        let stdout = self.stdout.take().unwrap();
        let (tx, rx) = mpsc::channel::<ProcessingChannelCommand>(100);
        self.processing_tx = Some(tx);

        let processing_container = ProcessingTaskParameters {
            url: String::from(url),
            stdout: Some(stdout),
            rx: rx
        };
        
        let processing_handler = tokio::spawn(async move {
            Self::draw(processing_container).await
        });
        self.processing_hander = Some(processing_handler);
        
        Ok(())
    }

    pub async fn to_saving_mode(&mut self) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
        let tx = self.processing_tx.as_mut().unwrap();
        tx.send(ProcessingChannelCommand::ToSaving).await?;

        Ok(())
    }

    async fn draw(mut container: ProcessingTaskParameters) -> Result<ProcessingTaskReturns, Box<dyn std::error::Error + Send + Sync>> {
        let mut stdout = container.stdout.take().unwrap();
        let mut processing_mode = "Compiling".cyan();

        let first_layer_msg = format!(" {} 🔗 {}", "Crawling".bold().cyan(), Self::pretty_url(&container.url, Self::PRETTIFY_MAX_LEN));
        writeln!(stdout, "{}", first_layer_msg)?;
        stdout.flush()?;

        
        let frames = ['|', '/', '-', '\\'];
        let mut second_layer_msg = String::new();
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

                        second_layer_msg = format!("   - {} {}", processing_mode, f);
                        crossterm::queue!(stdout, cursor::MoveTo(0, animation_row))?;
                        write!(stdout, "{}", second_layer_msg)?;
                        stdout.flush()?;
                    }
                    command = container.rx.recv() => {
                        match command {
                            Some(ProcessingChannelCommand::ToSaving) => {processing_mode = "Saving".green()},
                            Some(ProcessingChannelCommand::Done) => {
                                break 'animation_loop;
                            },
                            None => {
                                panic!("Channel closed!");
                            }
                        }
                    }
                }
            }
        }

        let processing_returns = ProcessingTaskReturns {
            stdout: Some(stdout),
            first_layer_msg: first_layer_msg,
            second_layer_msg: second_layer_msg,
            animation_row: animation_row
        };

        Ok(processing_returns)
    }

    async fn to_final_state(&mut self) -> Result<(Animator<FinalState>, ProcessingTaskReturns), Box<dyn std::error::Error + Send + Sync>> {
        let tx = self.processing_tx.as_mut().unwrap();
        tx.send(ProcessingChannelCommand::Done).await?;

        let processing_handler = self.processing_hander.take().unwrap();
        let mut processing_returns = processing_handler.await??;

        let terminal_out = processing_returns.stdout.take().unwrap();
        let animator = Animator {
            stdout: Some(terminal_out),
            state: std::marker::PhantomData::<FinalState>,
            processing_tx: None,
            processing_hander: None
        };
        Ok((animator, processing_returns))
    }
    
    pub async fn success(&mut self, url: &str) -> Result<Animator<FinalState>, Box<dyn std::error::Error + Send + Sync>> {
        let (mut animator, processing_returns) = self.to_final_state().await?;
        animator.do_success(processing_returns, url).await?;
        
        Ok(animator)
    }

    pub async fn failure(&mut self, url: &str, error_msg: Option<String>) -> Result<Animator<FinalState>, Box<dyn std::error::Error + Send + Sync>> {
        let (mut animator, processing_returns) = self.to_final_state().await?;
        animator.do_failure(processing_returns, url, error_msg).await?;

        Ok(animator)
    }
}

// ----------------- ANIMATOR - FinalState Implementation ----------------------
impl Animator<FinalState> {
    async fn do_success(&mut self, processing_returns: ProcessingTaskReturns, url: &str) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
        let stdout = self.stdout.as_mut().unwrap();
        crossterm::queue!(stdout, cursor::MoveTo(0, processing_returns.animation_row), terminal::Clear(terminal::ClearType::CurrentLine))?;
        crossterm::queue!(stdout, cursor::MoveTo(0, processing_returns.animation_row-1), terminal::Clear(terminal::ClearType::CurrentLine))?;
        write!(stdout, "{} 🔗 {}\n", "Success".green().bold(), Self::pretty_url(url, Self::PRETTIFY_MAX_LEN))?;
        stdout.flush()?;
        
        Ok(())
    }

    async fn do_failure(&mut self, processing_returns: ProcessingTaskReturns, url: &str, error_msg: Option<String>) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
        let stdout = self.stdout.as_mut().unwrap();
        crossterm::queue!(stdout, cursor::MoveTo(0, processing_returns.animation_row), terminal::Clear(terminal::ClearType::CurrentLine))?;
        let second_layer_msg_trim = {
            let mut second_layer_msg_trim: String = processing_returns
                .second_layer_msg
                .chars()
                .filter(|c| c.is_ascii())
                .collect();
            
            second_layer_msg_trim.pop();
            second_layer_msg_trim     
        };
        write!(stdout, "{}: {}", second_layer_msg_trim.red(), "failed".red())?;
        stdout.flush()?;
        tokio::time::sleep(tokio::time::Duration::from_millis(500)).await;

        crossterm::queue!(stdout, cursor::MoveTo(0, processing_returns.animation_row), terminal::Clear(terminal::ClearType::CurrentLine))?;
        crossterm::queue!(stdout, cursor::MoveTo(0, processing_returns.animation_row-1), terminal::Clear(terminal::ClearType::CurrentLine))?;
        write!(stdout, "{} 🔗 {}\n", "Failed".red().bold(), Self::pretty_url(url, Self::PRETTIFY_MAX_LEN))?;
        if let Some(error_msg) = error_msg {
            write!(stdout, "   {} {}\n", ">".cyan(), error_msg.red().italic())?;
        }
        stdout.flush()?;
        
        Ok(())
    }

    pub async fn reset(&mut self) -> Animator<InitialState> {
        let stdout = self.stdout.take().unwrap();
        Animator {
            stdout: Some(stdout),
            state: std::marker::PhantomData::<InitialState>,
            processing_tx: None,
            processing_hander: None
        }
    } 
}

// ----------------- ANIMATOR - HELPER Implementation ----------------------
impl<State> Animator<State> {
    const PRETTIFY_MAX_LEN: usize = 50;
    
    fn pretty_url(url: &str, max_len: usize) -> String {
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

        match leading.as_slice() {
            [] => collapsed,
            _ => format!("{}/{}/../{}", base, leading.join("/"), last),
        }
    }
}
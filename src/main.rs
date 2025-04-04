use crossterm::{
    cursor::MoveTo,
    cursor::{Hide, Show},
    event::{self, Event, KeyCode, KeyEvent, KeyModifiers},
    execute, queue,
    style::{Color, Print, ResetColor, SetBackgroundColor, SetForegroundColor},
    terminal::{EnterAlternateScreen, LeaveAlternateScreen, disable_raw_mode, enable_raw_mode},
};
use std::fs::{self, File};
use std::io::{self, BufRead, BufReader, Write};
use std::time::{Duration, Instant};
use std::{env, io::stdout};

// Simple editor mode enum
#[derive(PartialEq, Clone, Copy)]
enum Mode {
    Normal,
    Insert,
}

// Basic editor implementation
struct Editor {
    buffer: Vec<String>,
    cursor_x: usize,
    cursor_y: usize,
    mode: Mode,
    cursor_visible: bool,
    last_blink: Instant,
    blink_interval: Duration,
    filename: Option<String>,
}

impl Editor {
    fn new(filename: Option<String>) -> Self {
        let mut buffer = vec![String::new()];
        if let Some(ref name) = filename {
            if let Ok(file) = File::open(name) {
                buffer = BufReader::new(file)
                    .lines()
                    .filter_map(Result::ok)
                    .collect();
            }
        }

        Editor {
            buffer,
            cursor_x: 0,
            cursor_y: 0,
            mode: Mode::Normal,
            cursor_visible: true,
            last_blink: Instant::now(),
            blink_interval: Duration::from_millis(500), // Blink every 500ms
            filename,
        }
    }

    fn update_cursor_blink(&mut self) {
        let now = Instant::now();
        if now.duration_since(self.last_blink) >= self.blink_interval {
            self.cursor_visible = !self.cursor_visible;
            self.last_blink = now;
        }
    }

    fn render<W: Write>(&mut self, out: &mut W) -> Result<(), Box<dyn std::error::Error>> {
        // Update cursor blinking state
        self.update_cursor_blink();

        // Clear screen and reset cursor
        queue!(
            out,
            crossterm::terminal::Clear(crossterm::terminal::ClearType::All)
        )?;
        queue!(out, MoveTo(0, 0))?;

        // Render buffer
        for (y, line) in self.buffer.iter().enumerate() {
            queue!(out, MoveTo(0, y as u16))?;

            if y == self.cursor_y {
                // Render line with cursor
                for (x, ch) in line.chars().enumerate() {
                    if x == self.cursor_x && self.cursor_visible {
                        // Draw character with cursor highlighting
                        match self.mode {
                            Mode::Normal => {
                                // Block cursor (inverted colors)
                                queue!(
                                    out,
                                    SetBackgroundColor(Color::White),
                                    SetForegroundColor(Color::Black),
                                    Print(ch),
                                    ResetColor
                                )?;
                            }
                            Mode::Insert => {
                                // Vertical bar cursor (character + bar)
                                queue!(
                                    out,
                                    Print(ch),
                                    MoveTo(x as u16, y as u16),
                                    SetBackgroundColor(Color::White),
                                    Print("|"),
                                    ResetColor,
                                    MoveTo(x as u16 + 1, y as u16)
                                )?;
                            }
                        }
                    } else {
                        // Regular character
                        queue!(out, Print(ch))?;
                    }
                }

                // Handle cursor at end of line
                if self.cursor_x >= line.len() && self.cursor_visible {
                    match self.mode {
                        Mode::Normal => {
                            queue!(
                                out,
                                SetBackgroundColor(Color::White),
                                Print(" "),
                                ResetColor
                            )?;
                        }
                        Mode::Insert => {
                            queue!(
                                out,
                                SetBackgroundColor(Color::White),
                                Print("|"),
                                ResetColor
                            )?;
                        }
                    }
                }
            } else {
                // Render line normally
                queue!(out, Print(line))?;
            }
        }

        // Render status line
        let mode_str = match self.mode {
            Mode::Normal => "NORMAL",
            Mode::Insert => "INSERT",
        };

        let (_term_width, term_height) = crossterm::terminal::size()?;
        let status = format!(
            "{} | Line: {}, Col: {} ",
            mode_str,
            self.cursor_y + 1,
            self.cursor_x + 1
        );
        queue!(
            out,
            MoveTo(0, term_height - 2),
            SetBackgroundColor(Color::Blue),
            SetForegroundColor(Color::White),
            Print(status),
            ResetColor
        )?;

        // Hide the terminal cursor since we're drawing our own
        queue!(out, Hide)?;

        out.flush()?;
        Ok(())
    }

    fn handle_normal_key(&mut self, key: KeyCode) -> bool {
        match key {
            // Mode switching
            KeyCode::Char('i') => {
                self.mode = Mode::Insert;
                false
            }

            // Movement
            KeyCode::Char('h') | KeyCode::Left => {
                if self.cursor_x > 0 {
                    self.cursor_x -= 1;
                }
                false
            }
            KeyCode::Char('j') | KeyCode::Down => {
                if self.cursor_y < self.buffer.len() - 1 {
                    self.cursor_y += 1;
                    // Adjust x if necessary
                    let line_len = self.buffer[self.cursor_y].len();
                    if self.cursor_x > line_len {
                        self.cursor_x = line_len;
                    }
                }
                false
            }
            KeyCode::Char('k') | KeyCode::Up => {
                if self.cursor_y > 0 {
                    self.cursor_y -= 1;
                    // Adjust x if necessary
                    let line_len = self.buffer[self.cursor_y].len();
                    if self.cursor_x > line_len {
                        self.cursor_x = line_len;
                    }
                }
                false
            }
            KeyCode::Char('l') | KeyCode::Right => {
                let line_len = self.buffer[self.cursor_y].len();
                if self.cursor_x < line_len {
                    self.cursor_x += 1;
                }
                false
            }

            // Start/end of line
            KeyCode::Char('0') => {
                self.cursor_x = 0;
                false
            }
            KeyCode::Char('$') => {
                let line_len = self.buffer[self.cursor_y].len();
                self.cursor_x = if line_len > 0 { line_len } else { 0 };
                false
            }
            KeyCode::Char(':') => {
                stdout().flush().unwrap();

                self.prompt_and_execute_command();
                false
            }
            // Quit
            // move this to command executor at some point
            KeyCode::Char('q') => true,
            _ => false,
        }
    }

    fn prompt_and_execute_command(&mut self) {
        io::stdout().flush().unwrap();

        disable_raw_mode().unwrap();
        print!(": ");

        io::stdout().flush().unwrap();

        let mut command = String::new();
        io::stdin().read_line(&mut command).unwrap();

        io::stdout().flush().unwrap();

        let command = command.trim();
        match command {
            "w" => self.save_buffer(),
            "q" => std::process::exit(0),
            "wq" => {
                self.save_buffer();
                std::process::exit(0);
            }
            _ => {
                println!("Unsupported or unknown command: {}", command);
            }
        }

        enable_raw_mode().unwrap();
    }

    fn save_buffer(&mut self) {
        let filename = match &self.filename {
            Some(name) => name.clone(),
            None => {
                print!("Enter filename: ");
                io::stdout().flush().unwrap(); // Flush to ensure prompt appears

                let mut name = String::new();
                if io::stdin().read_line(&mut name).is_err() {
                    println!("Failed to read filename.");
                    return;
                }

                let trimmed_name = name.trim().to_string();
                if trimmed_name.is_empty() {
                    println!("Filename cannot be empty!");
                    return;
                }

                trimmed_name
            }
        };

        match File::create(&filename) {
            Ok(mut file) => {
                for line in &self.buffer {
                    if writeln!(file, "{}", line).is_err() {
                        println!("Failed to write to file.");
                        return;
                    }
                }

                println!("File saved: {}", filename);
                self.filename = Some(filename);
            }
            Err(err) => {
                println!("Failed to create file: {}", err);
            }
        }
    }
    fn handle_insert_key(&mut self, key: KeyCode) -> bool {
        match key {
            KeyCode::Esc => {
                self.mode = Mode::Normal;
                // Adjust cursor if at end of line
                let line_len = self.buffer[self.cursor_y].len();
                if line_len > 0 && self.cursor_x >= line_len {
                    self.cursor_x = line_len - 1;
                }
                false
            }

            KeyCode::Char(c) => {
                // Ensure the current line is long enough
                let line = &mut self.buffer[self.cursor_y];

                // Insert character
                if self.cursor_x >= line.len() {
                    line.push(c);
                } else {
                    line.insert(self.cursor_x, c);
                }

                self.cursor_x += 1;
                false
            }

            KeyCode::Backspace => {
                if self.cursor_x > 0 {
                    let line = &mut self.buffer[self.cursor_y];
                    line.remove(self.cursor_x - 1);
                    self.cursor_x -= 1;
                } else if self.cursor_y > 0 {
                    // Join with previous line
                    let current_line = self.buffer.remove(self.cursor_y);
                    self.cursor_y -= 1;
                    self.cursor_x = self.buffer[self.cursor_y].len();
                    self.buffer[self.cursor_y].push_str(&current_line);
                }
                false
            }

            KeyCode::Enter => {
                // Split line at cursor
                let line = &mut self.buffer[self.cursor_y];
                let new_line = if self.cursor_x < line.len() {
                    line.split_off(self.cursor_x)
                } else {
                    String::new()
                };

                // Insert new line
                self.buffer.insert(self.cursor_y + 1, new_line);
                self.cursor_y += 1;
                self.cursor_x = 0;
                false
            }

            // Basic movement
            KeyCode::Left => {
                if self.cursor_x > 0 {
                    self.cursor_x -= 1;
                }
                false
            }
            KeyCode::Right => {
                let line_len = self.buffer[self.cursor_y].len();
                if self.cursor_x < line_len {
                    self.cursor_x += 1;
                }
                false
            }
            KeyCode::Up => {
                if self.cursor_y > 0 {
                    self.cursor_y -= 1;
                    let line_len = self.buffer[self.cursor_y].len();
                    if self.cursor_x > line_len {
                        self.cursor_x = line_len;
                    }
                }
                false
            }
            KeyCode::Down => {
                if self.cursor_y < self.buffer.len() - 1 {
                    self.cursor_y += 1;
                    let line_len = self.buffer[self.cursor_y].len();
                    if self.cursor_x > line_len {
                        self.cursor_x = line_len;
                    }
                }
                false
            }

            _ => false,
        }
    }

    fn handle_keypress(&mut self, key: KeyCode) -> bool {
        match self.mode {
            Mode::Normal => self.handle_normal_key(key),
            Mode::Insert => self.handle_insert_key(key),
        }
    }
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Setup terminal
    enable_raw_mode()?;
    let mut stdout = stdout();
    execute!(stdout, EnterAlternateScreen, Hide)?;

    let args: Vec<String> = env::args().collect();
    let filename = args.get(1).cloned();

    // Create editor
    let mut editor = Editor::new(filename);

    // Main loop
    let mut should_quit = false;
    while !should_quit {
        // Render current state
        editor.render(&mut stdout)?;

        // Handle input with timeout to allow cursor blinking
        if event::poll(Duration::from_millis(16))? {
            // ~60fps for smooth blinking
            if let Event::Key(KeyEvent {
                code, modifiers, ..
            }) = event::read()?
            {
                // Check for Ctrl+C to quit
                if code == KeyCode::Char('c') && modifiers.contains(KeyModifiers::CONTROL) {
                    should_quit = true;
                } else {
                    // Process regular keypress
                    should_quit = editor.handle_keypress(code);
                }
            }
        }
    }

    // Cleanup terminal
    disable_raw_mode()?;
    execute!(stdout, LeaveAlternateScreen, Show)?;

    Ok(())
}

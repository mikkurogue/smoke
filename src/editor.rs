use crossterm::{
    cursor::{Hide, MoveTo},
    event::KeyCode,
    queue,
    style::{Color, Print, ResetColor, SetBackgroundColor, SetForegroundColor},
    terminal::{disable_raw_mode, enable_raw_mode},
};
use std::{
    fs::File,
    io::{BufRead, BufReader, Write, stdin, stdout},
};

use crate::{cursor::Cursor, mode::Mode};

pub struct Editor {
    buffer: Vec<String>,
    cursor: Cursor,
    mode: Mode,
    filename: Option<String>,
}

impl Editor {
    pub fn new(filename: Option<String>) -> Self {
        let mut buffer = vec![String::new()];
        if let Some(ref name) = filename {
            if let Ok(file) = File::open(name) {
                buffer = BufReader::new(file)
                    .lines()
                    .filter_map(Result::ok)
                    .collect();
            }
        }

        let cursor = Cursor::new();

        Editor {
            buffer,
            cursor,
            mode: Mode::Normal,
            filename,
        }
    }

    pub fn handle_keypress(&mut self, key: KeyCode) -> bool {
        match self.mode {
            Mode::Normal => self.handle_normal_key(key),
            Mode::Insert => self.handle_insert_key(key),
        }
    }

    pub fn render<W: Write>(&mut self, out: &mut W) -> Result<(), Box<dyn std::error::Error>> {
        self.cursor.blink();

        queue!(
            out,
            crossterm::terminal::Clear(crossterm::terminal::ClearType::All)
        )?;
        queue!(out, MoveTo(0, 0))?;

        for (y, line) in self.buffer.iter().enumerate() {
            queue!(out, MoveTo(0, y as u16))?;

            if y == self.cursor.x {
                for (x, ch) in line.chars().enumerate() {
                    if x == self.cursor.x && self.cursor.visible {
                        match self.mode {
                            Mode::Normal => {
                                queue!(
                                    out,
                                    SetBackgroundColor(Color::White),
                                    SetForegroundColor(Color::Black),
                                    Print(ch),
                                    ResetColor
                                )?;
                            }
                            Mode::Insert => {
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
                        queue!(out, Print(ch))?;
                    }
                }

                if self.cursor.x >= line.len() && self.cursor.visible {
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

        let mode_str = match self.mode {
            Mode::Normal => "NORMAL",
            Mode::Insert => "INSERT",
        };

        let (_term_width, term_height) = crossterm::terminal::size()?;
        let status = format!(
            "{} | Line: {}, Col: {} ",
            mode_str,
            self.cursor.y + 1,
            self.cursor.x + 1
        );
        queue!(
            out,
            MoveTo(0, term_height - 2),
            SetBackgroundColor(Color::Blue),
            SetForegroundColor(Color::White),
            Print(status),
            ResetColor
        )?;

        queue!(out, Hide)?;

        out.flush()?;
        Ok(())
    }

    fn handle_normal_key(&mut self, key: KeyCode) -> bool {
        match key {
            KeyCode::Char('i') => {
                self.mode = Mode::Insert;
                false
            }

            KeyCode::Char('h') | KeyCode::Left => {
                if self.cursor.x > 0 {
                    self.cursor.x -= 1;
                }
                false
            }
            KeyCode::Char('j') | KeyCode::Down => {
                if self.cursor.y < self.buffer.len() - 1 {
                    self.cursor.y += 1;
                    // Adjust x if necessary
                    let line_len = self.buffer[self.cursor.y].len();
                    if self.cursor.x > line_len {
                        self.cursor.x = line_len;
                    }
                }
                false
            }
            KeyCode::Char('k') | KeyCode::Up => {
                if self.cursor.y > 0 {
                    self.cursor.y -= 1;
                    let line_len = self.buffer[self.cursor.y].len();
                    if self.cursor.x > line_len {
                        self.cursor.x = line_len;
                    }
                }
                false
            }
            KeyCode::Char('l') | KeyCode::Right => {
                let line_len = self.buffer[self.cursor.y].len();
                if self.cursor.x < line_len {
                    self.cursor.x += 1;
                }
                false
            }

            KeyCode::Char('0') => {
                self.cursor.x = 0;
                false
            }
            KeyCode::Char('$') => {
                let line_len = self.buffer[self.cursor.y].len();
                self.cursor.x = if line_len > 0 { line_len } else { 0 };
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
        stdout().flush().unwrap();

        disable_raw_mode().unwrap();
        print!(": ");

        stdout().flush().unwrap();

        let mut command = String::new();
        stdin().read_line(&mut command).unwrap();

        stdout().flush().unwrap();

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
                stdout().flush().unwrap();
                let mut name = String::new();
                if stdin().read_line(&mut name).is_err() {
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
                let line_len = self.buffer[self.cursor.y].len();
                if line_len > 0 && self.cursor.x >= line_len {
                    self.cursor.x = line_len - 1;
                }
                false
            }

            KeyCode::Char(c) => {
                let line = &mut self.buffer[self.cursor.y];

                if self.cursor.x >= line.len() {
                    line.push(c);
                } else {
                    line.insert(self.cursor.x, c);
                }

                self.cursor.x += 1;
                false
            }

            KeyCode::Backspace => {
                if self.cursor.x > 0 {
                    let line = &mut self.buffer[self.cursor.y];
                    line.remove(self.cursor.x - 1);
                    self.cursor.x -= 1;
                } else if self.cursor.y > 0 {
                    let current_line = self.buffer.remove(self.cursor.y);
                    self.cursor.y -= 1;
                    self.cursor.x = self.buffer[self.cursor.y].len();
                    self.buffer[self.cursor.y].push_str(&current_line);
                }
                false
            }

            KeyCode::Enter => {
                let line = &mut self.buffer[self.cursor.y];
                let new_line = if self.cursor.x < line.len() {
                    line.split_off(self.cursor.x)
                } else {
                    String::new()
                };

                self.buffer.insert(self.cursor.y + 1, new_line);
                self.cursor.y += 1;
                self.cursor.x = 0;
                false
            }

            KeyCode::Left => {
                if self.cursor.x > 0 {
                    self.cursor.x -= 1;
                }
                false
            }
            KeyCode::Right => {
                let line_len = self.buffer[self.cursor.y].len();
                if self.cursor.x < line_len {
                    self.cursor.x += 1;
                }
                false
            }
            KeyCode::Up => {
                if self.cursor.y > 0 {
                    self.cursor.y -= 1;
                    let line_len = self.buffer[self.cursor.y].len();
                    if self.cursor.x > line_len {
                        self.cursor.x = line_len;
                    }
                }
                false
            }
            KeyCode::Down => {
                if self.cursor.y < self.buffer.len() - 1 {
                    self.cursor.y += 1;
                    let line_len = self.buffer[self.cursor.y].len();
                    if self.cursor.x > line_len {
                        self.cursor.x = line_len;
                    }
                }
                false
            }

            _ => false,
        }
    }
}

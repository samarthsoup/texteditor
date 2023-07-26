use crossterm::event::*;
use std::io::{Write, stdout, ErrorKind};
use std::{cmp, env, fs}; 
use std::path::PathBuf;

use crate::reader::Reader;
use crate::prompt;
use crate::output::Output;

const TAB_STOP: usize = 8;
const QUIT_TIMES: u8 = 3;

pub struct EditorContents {
    pub content: String,
}

impl EditorContents {
    pub fn new() -> Self {
        Self {
            content: String::new(),
        }
    }
    
    pub fn push(&mut self, ch: char) {
        self.content.push(ch)
    }

    pub fn push_str(&mut self, string: &str) {
        self.content.push_str(string)
    }
}

impl std::io::Write for EditorContents {
    fn write(&mut self, buf: &[u8]) -> std::io::Result<usize> {
        match std::str::from_utf8(buf) {
            Ok(s) => {
                self.content.push_str(s);
                Ok(s.len())
            }
            Err(_) => Err(std::io::ErrorKind::WriteZero.into()),
        }
    }

    fn flush(&mut self) -> std::io::Result<()> {
        let out = write!(stdout(), "{}", self.content);
        stdout().flush()?;
        self.content.clear();
        out
    }
}

#[derive(Default)] 
pub struct Row {
    pub row_content: String,
    pub render: String,
}

impl Row {
    pub fn new(row_content: String, render: String) -> Self {
        Self {
            row_content,
            render,
        }
    }

    pub fn insert_char(&mut self, at: usize, ch: char) {
        self.row_content.insert(at, ch);
        EditorRows::render_row(self)
    }

    pub fn delete_char(&mut self, at: usize) {
        self.row_content.remove(at);
        EditorRows::render_row(self)
    }

    pub fn get_row_content_x(&self, render_x: usize) -> usize {
        let mut current_render_x = 0;
        for (cursor_x, ch) in self.row_content.chars().enumerate() {
            if ch == '\t' {
                current_render_x += (TAB_STOP - 1) - (current_render_x % TAB_STOP);
            }
            current_render_x += 1;
            if current_render_x > render_x {
                return cursor_x;
            }
        }
        0
    }
}

pub struct EditorRows {
    pub row_contents: Vec<Row>,
    pub filename: Option<PathBuf>
}

impl EditorRows {
    pub fn new() -> Self {
        match env::args().nth(1) {
            None => Self {
                row_contents: Vec::new(),
                filename: None, 
            },
            Some(file) => Self::from_file(file.into()), 
        }
    }

    pub fn from_file(file: PathBuf) -> Self {
        let file_contents = fs::read_to_string(&file).expect("Unable to read file"); 
        Self {
            filename: Some(file), 
            row_contents: file_contents
                .lines()
                .map(|it| {
                    let mut row = Row::new(it.into(), String::new());
                    Self::render_row(&mut row);
                    row
                })
                .collect(),
        }
    }
    
    pub fn save(&self) -> std::io::Result<usize> { 
        match &self.filename {
            None => Err(std::io::Error::new(ErrorKind::Other, "no file name specified")),
            Some(name) => {
                let mut file = fs::OpenOptions::new().write(true).create(true).open(name)?;
                let contents: String = self
                    .row_contents
                    .iter()
                    .map(|it| it.row_content.as_str())
                    .collect::<Vec<&str>>()
                    .join("\n");
                file.set_len(contents.len() as u64)?;
                file.write_all(contents.as_bytes())?; 
                Ok(contents.as_bytes().len()) 
            }
        }
    }

    pub fn get_render(&self, at: usize) -> &String {
        &self.row_contents[at].render
    }
    
    
    pub fn get_editor_row(&self, at: usize) -> &Row {
        &self.row_contents[at]
    }

    pub fn number_of_rows(&self) -> usize {
        self.row_contents.len() 
    }

    pub fn get_row(&self, at: usize) -> &str {
        &self.row_contents[at].row_content
    }

    pub fn insert_row(&mut self, at: usize, contents: String) {
        let mut new_row = Row::new(contents, String::new());
        EditorRows::render_row(&mut new_row);
        self.row_contents.insert(at, new_row);
    }

    pub fn get_editor_row_mut(&mut self, at: usize) -> &mut Row {
        &mut self.row_contents[at]
    }

    pub fn render_row(row: &mut Row) {
        let mut index = 0;
        let capacity = row
            .row_content
            .chars()
            .fold(0, |acc, next| acc + if next == '\t' { TAB_STOP } else { 1 });
        row.render = String::with_capacity(capacity);
        row.row_content.chars().for_each(|c| {
            index += 1;
            if c == '\t' {
                row.render.push(' ');
                while index % TAB_STOP != 0 { 
                    row.render.push(' ');
                    index += 1
                }
            } else {
                row.render.push(c);
            }
        });
    }

    pub fn join_adjacent_rows(&mut self, at: usize) {
        let current_row = self.row_contents.remove(at);
        let previous_row = self.get_editor_row_mut(at - 1);
        previous_row.row_content.push_str(&current_row.row_content);
        Self::render_row(previous_row);
    }
}

pub struct Editor {
    pub reader: Reader,
    pub output: Output,
    pub quit_times: u8
}


impl Editor {
    pub fn new() -> Self {
        Self {
            reader: Reader,
            output: Output::new(),
            quit_times: QUIT_TIMES, 
        }
    }

    pub fn process_keypress(&mut self) -> crossterm::Result<bool> {
        match self.reader.read_key()? {
            KeyEvent {
                code: KeyCode::Char('q'),
                modifiers: KeyModifiers::CONTROL,
            } => {
                if self.output.dirty > 0 && self.quit_times > 0 {
                    self.output.status_message.set_message(format!(
                        "WARNING!!! File has unsaved changes. Press Ctrl-Q {} more times to quit.",
                        self.quit_times
                    ));
                    self.quit_times -= 1;
                    return Ok(true);
                }
                return Ok(false);
            }
            KeyEvent {
                code:
                    direction
                    @
                    (KeyCode::Up
                    | KeyCode::Down
                    | KeyCode::Left
                    | KeyCode::Right
                    | KeyCode::Home
                    | KeyCode::End),
                modifiers: KeyModifiers::NONE,
            } => self.output.move_cursor(direction),
            KeyEvent {
                code: val @ (KeyCode::PageUp | KeyCode::PageDown),
                modifiers: KeyModifiers::NONE,
            } => {
                if matches!(val, KeyCode::PageUp) {
                    self.output.cursor_controller.cursor_y =
                        self.output.cursor_controller.row_offset
                } else {
                    self.output.cursor_controller.cursor_y = cmp::min(
                        self.output.win_size.1 + self.output.cursor_controller.row_offset - 1,
                        self.output.editor_rows.number_of_rows(),
                    );
                }
                (0..self.output.win_size.1).for_each(|_| {
                    self.output.move_cursor(if matches!(val, KeyCode::PageUp) {
                        KeyCode::Up
                    } else {
                        KeyCode::Down
                    });
                })
            }
            KeyEvent {
                code: KeyCode::Char('s'),
                modifiers: KeyModifiers::CONTROL,
            } => {
                if matches!(self.output.editor_rows.filename, None) {
                    let prompt = prompt!(&mut self.output, "Save as : {} (ESC to cancel)")
                        .map(|it| it.into());
                    if let None = prompt {
                        self.output
                            .status_message
                            .set_message("Save Aborted".into());
                        return Ok(true);
                    }
                    self.output.editor_rows.filename = prompt
                }
                self.output.editor_rows.save().map(|len| {
                    self.output
                        .status_message
                        .set_message(format!("{} bytes written to disk", len));
                    self.output.dirty = 0
                })?;
            }
            KeyEvent {
                code: KeyCode::Char('f'),
                modifiers: KeyModifiers::CONTROL,
            } => {
                self.output.find()?;
            }
            KeyEvent {
                code: key @ (KeyCode::Backspace | KeyCode::Delete),
                modifiers: KeyModifiers::NONE,
            } => {
                if matches!(key, KeyCode::Delete) {
                    self.output.move_cursor(KeyCode::Right)
                }
                self.output.delete_char()
            }
            KeyEvent {
                code: KeyCode::Enter,
                modifiers: KeyModifiers::NONE,
            } => self.output.insert_newline(),
            KeyEvent {
                code: code @ (KeyCode::Char(..) | KeyCode::Tab),
                modifiers: KeyModifiers::NONE | KeyModifiers::SHIFT,
            } => self.output.insert_char(match code {
                KeyCode::Tab => '\t',
                KeyCode::Char(ch) => ch,
                _ => unreachable!(),
            }),
            _ => {}
        }
        self.quit_times = QUIT_TIMES;
        Ok(true)
    }

    pub fn run(&mut self) -> crossterm::Result<bool> {
        self.output.refresh_screen()?;
        self.process_keypress()
    }
}
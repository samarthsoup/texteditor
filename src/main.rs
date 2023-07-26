use crossterm::terminal;

mod reader;
mod search;
mod output;
mod editor;

use crate::output::Output;
use crate::editor::{EditorContents, EditorRows, Editor};

#[macro_export]
macro_rules! prompt {
    ($output:expr,$args:tt) => {
        prompt!($output, $args, callback = |&_, _, _| {})
    };
    ($output:expr,$args:tt, callback = $callback:expr) => {{
        let output: &mut Output = $output;
        let mut input = String::with_capacity(32);
        loop {
            output.status_message.set_message(format!($args, input)); 
            output.refresh_screen()?;
            let key_event = Reader.read_key()?;
            match key_event {
                KeyEvent {
                    code: KeyCode::Enter,
                    modifiers: KeyModifiers::NONE,
                } => {
                    if !input.is_empty() {
                        output.status_message.set_message(String::new());
                        $callback(output, &input, KeyCode::Enter); 
                        break;
                    }
                }
                KeyEvent {
                    code: KeyCode::Esc, ..
                } => {
                    output.status_message.set_message(String::new());
                    input.clear();
                    $callback(output, &input, KeyCode::Esc); 
                    break;
                }
                KeyEvent {
                    code: KeyCode::Backspace | KeyCode::Delete,
                    modifiers: KeyModifiers::NONE,
                } => {
                    input.pop();
                }
                KeyEvent {
                    code: code @ (KeyCode::Char(..) | KeyCode::Tab),
                    modifiers: KeyModifiers::NONE | KeyModifiers::SHIFT,
                } => {
                    input.push(match code {
                        KeyCode::Tab => '\t',
                        KeyCode::Char(ch) => ch,
                        _ => unreachable!(),
                    });
                }
                _ => {}
            }
            $callback(output, &input, key_event.code); 
        }
        if input.is_empty() {
            None
        } else {
            Some(input)
        }
    }};
}

struct CleanUp;

impl Drop for CleanUp {
    fn drop(&mut self) {
        terminal::disable_raw_mode().expect("unable to disable raw mode");
        Output::clear_screen().expect("couldn't clear screen");
    }
}

fn main() -> crossterm::Result<()> {
    let _clean_up = CleanUp;
    terminal::enable_raw_mode()?;
    let mut editor = Editor::new();
    while editor.run()? {}
    Ok(())
}
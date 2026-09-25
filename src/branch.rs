use crate::git::{self, Branch};
use anyhow::Result;
use crossterm::{
    cursor,
    event::{self, Event, KeyCode, KeyEvent, KeyEventKind, KeyModifiers},
    execute, queue,
    style::{Attribute, Color, Print, ResetColor, SetAttribute, SetForegroundColor},
    terminal::{self, ClearType},
};
use std::io::{Write, stdout};

/// Restores the terminal (and erases the inline UI) even on early return or panic.
/// The cursor is always kept at the top-left of the UI region between draws.
struct TerminalGuard;

impl TerminalGuard {
    fn new() -> Result<Self> {
        terminal::enable_raw_mode()?;
        execute!(stdout(), cursor::Hide)?;
        Ok(TerminalGuard)
    }
}

impl Drop for TerminalGuard {
    fn drop(&mut self) {
        let _ = execute!(
            stdout(),
            cursor::MoveToColumn(0),
            terminal::Clear(ClearType::FromCursorDown),
            cursor::Show
        );
        let _ = terminal::disable_raw_mode();
    }
}

enum Outcome {
    Quit,
    Switch(String),
}

struct State {
    branches: Vec<Branch>,
    selected: usize,
    offset: usize,
    message: Option<(bool, String)>,
}

impl State {
    fn reload(&mut self) -> Result<()> {
        self.branches = git::branches()?;
        self.selected = self.selected.min(self.branches.len().saturating_sub(1));
        Ok(())
    }
}

pub fn run() -> Result<i32> {
    let branches = git::branches()?;
    if branches.is_empty() {
        eprintln!("No local branches.");
        return Ok(1);
    }
    let mut state = State {
        branches,
        selected: 0,
        offset: 0,
        message: None,
    };

    let outcome = {
        let _guard = TerminalGuard::new()?;
        event_loop(&mut state)?
    };

    match outcome {
        Outcome::Quit => Ok(0),
        Outcome::Switch(name) => git::switch(&name),
    }
}

fn event_loop(state: &mut State) -> Result<Outcome> {
    loop {
        draw(state)?;
        let Event::Key(KeyEvent {
            code,
            modifiers,
            kind: KeyEventKind::Press,
            ..
        }) = event::read()?
        else {
            continue;
        };
        let last = state.branches.len().saturating_sub(1);
        match code {
            KeyCode::Char('c') if modifiers.contains(KeyModifiers::CONTROL) => {
                return Ok(Outcome::Quit);
            }
            KeyCode::Char('q') | KeyCode::Esc => return Ok(Outcome::Quit),
            KeyCode::Up | KeyCode::Char('k') => state.selected = state.selected.saturating_sub(1),
            KeyCode::Down | KeyCode::Char('j') => state.selected = (state.selected + 1).min(last),
            KeyCode::Home | KeyCode::Char('g') => state.selected = 0,
            KeyCode::End | KeyCode::Char('G') => state.selected = last,
            KeyCode::Enter => {
                if let Some(b) = state.branches.get(state.selected) {
                    return Ok(Outcome::Switch(b.name.clone()));
                }
            }
            KeyCode::Char(c @ ('d' | 'D')) => {
                if let Some(b) = state.branches.get(state.selected) {
                    state.message = Some(git::delete_branch(&b.name, c == 'D')?);
                    state.reload()?;
                    if state.branches.is_empty() {
                        return Ok(Outcome::Quit);
                    }
                }
            }
            _ => {}
        }
    }
}

fn draw(state: &mut State) -> Result<()> {
    let (width, height) = match terminal::size()? {
        (0, _) | (_, 0) => (80, 24),
        size => size,
    };
    let width = width as usize;
    let footer_lines = if state.message.is_some() { 2 } else { 1 };
    // Leave one spare row so the prompt line above stays visible.
    let max_list = (height as usize).saturating_sub(footer_lines + 1).max(1);
    let list_height = state.branches.len().min(max_list);

    if state.selected < state.offset {
        state.offset = state.selected;
    } else if state.selected >= state.offset + list_height {
        state.offset = state.selected + 1 - list_height;
    }

    let mut out = stdout();
    queue!(out, cursor::MoveToColumn(0), terminal::Clear(ClearType::FromCursorDown))?;

    let visible = state.branches.iter().enumerate().skip(state.offset).take(list_height);
    for (i, b) in visible {
        let marker = if b.is_current { "* " } else { "  " };
        let line = truncate(&format!("{marker}{}", b.name), width.saturating_sub(2));
        if i == state.selected {
            queue!(
                out,
                SetForegroundColor(Color::Cyan),
                SetAttribute(Attribute::Reverse),
                Print(format!("> {line}")),
                SetAttribute(Attribute::Reset),
                ResetColor,
            )?;
        } else if b.is_current {
            queue!(out, SetForegroundColor(Color::Green), Print(format!("  {line}")), ResetColor)?;
        } else {
            queue!(out, Print(format!("  {line}")))?;
        }
        queue!(out, Print("\r\n"))?;
    }

    if let Some((ok, msg)) = &state.message {
        let color = if *ok { Color::Green } else { Color::Red };
        let msg = msg.lines().next().unwrap_or("");
        queue!(out, SetForegroundColor(color), Print(truncate(msg, width)), ResetColor, Print("\r\n"))?;
    }
    queue!(
        out,
        SetForegroundColor(Color::DarkGrey),
        Print(truncate(
            "↑/k ↓/j move · enter switch · d delete · D force delete · q quit",
            width
        )),
        ResetColor,
    )?;

    // Return to the top of the region; relative moves stay correct even if printing scrolled.
    let drawn = list_height + footer_lines;
    queue!(out, cursor::MoveToColumn(0))?;
    if drawn > 1 {
        queue!(out, cursor::MoveUp((drawn - 1) as u16))?;
    }
    out.flush()?;
    Ok(())
}

fn truncate(s: &str, max: usize) -> String {
    s.chars().take(max).collect()
}

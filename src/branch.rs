use crate::git::{self, Branch};
use crate::ui::{LineEditor, to_u16, to_u32, truncate};
use anyhow::Result;
use crossterm::{
    cursor,
    event::{self, Event, KeyCode, KeyEvent, KeyEventKind, KeyModifiers},
    execute, queue,
    style::{Attribute, Color, Print, ResetColor, SetAttribute, SetForegroundColor},
    terminal::{self, ClearType},
};
use nucleo_matcher::{
    Config, Matcher, Utf32Str,
    pattern::{CaseMatching, Normalization, Pattern},
};
use std::cmp::Reverse;
use std::io::{Write, stdout};

/// Restores the terminal (and erases the inline UI) even on early return or panic.
/// The cursor is always kept at the top-left of the UI region between draws.
struct TerminalGuard;

impl TerminalGuard {
    fn new() -> Result<Self> {
        terminal::enable_raw_mode()?;
        execute!(stdout(), cursor::Hide)?;
        Ok(Self)
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
    Switch(String, Option<String>),
}

/// A branch that passed the current search, with the char positions of the label to highlight.
struct Match {
    index: usize,
    positions: Vec<u32>,
}

struct State {
    branches: Vec<Branch>,
    /// Branches to display, best match first (all branches in order when not searching).
    view: Vec<Match>,
    include_remote: bool,
    searching: bool,
    query: LineEditor,
    matcher: Matcher,
    selected: usize,
    offset: usize,
    message: Option<(bool, String)>,
}

fn label(b: &Branch) -> &str {
    b.remote.as_deref().unwrap_or(&b.name)
}

impl State {
    fn reload(&mut self) -> Result<()> {
        self.branches = git::branches(self.include_remote)?;
        self.refilter();
        Ok(())
    }

    /// Recomputes `view` from `branches` and `query`, keeping the selection in range.
    fn refilter(&mut self) {
        let query = self.query.text();
        if query.is_empty() {
            self.view = (0..self.branches.len())
                .map(|index| Match {
                    index,
                    positions: Vec::new(),
                })
                .collect();
        } else {
            let pattern = Pattern::parse(&query, CaseMatching::Smart, Normalization::Smart);
            let mut buf = Vec::new();
            let mut scored = Vec::new();
            for (index, b) in self.branches.iter().enumerate() {
                let mut positions = Vec::new();
                let haystack = Utf32Str::new(label(b), &mut buf);
                if let Some(score) = pattern.indices(haystack, &mut self.matcher, &mut positions) {
                    positions.sort_unstable();
                    positions.dedup();
                    scored.push((score, Match { index, positions }));
                }
            }
            // Stable sort: equal scores keep the original branch order.
            scored.sort_by_key(|(score, _)| Reverse(*score));
            self.view = scored.into_iter().map(|(_, m)| m).collect();
        }
        self.selected = self.selected.min(self.view.len().saturating_sub(1));
    }

    /// Called after the query text changed.
    fn query_changed(&mut self) {
        self.selected = 0;
        self.offset = 0;
        self.refilter();
    }

    fn selected_branch(&self) -> Option<&Branch> {
        self.view
            .get(self.selected)
            .map(|m| &self.branches[m.index])
    }
}

pub fn run() -> Result<i32> {
    let branches = git::branches(false)?;
    if branches.is_empty() {
        eprintln!("No local branches.");
        return Ok(1);
    }
    let mut state = State {
        view: Vec::new(),
        branches,
        include_remote: false,
        searching: false,
        query: LineEditor::default(),
        matcher: Matcher::new(Config::DEFAULT),
        selected: 0,
        offset: 0,
        message: None,
    };
    state.refilter();

    let outcome = {
        let _guard = TerminalGuard::new()?;
        event_loop(&mut state)?
    };

    match outcome {
        Outcome::Quit => Ok(0),
        Outcome::Switch(name, remote) => git::switch(&name, remote.as_deref()),
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
        let last = state.view.len().saturating_sub(1);
        if state.searching {
            match code {
                KeyCode::Char('c') if modifiers.contains(KeyModifiers::CONTROL) => {
                    return Ok(Outcome::Quit);
                }
                KeyCode::Esc => {
                    state.searching = false;
                    state.query.clear();
                    state.query_changed();
                }
                KeyCode::Enter => {
                    if let Some(b) = state.selected_branch() {
                        return Ok(Outcome::Switch(b.name.clone(), b.remote.clone()));
                    }
                }
                KeyCode::Up => state.selected = state.selected.saturating_sub(1),
                KeyCode::Down => state.selected = (state.selected + 1).min(last),
                KeyCode::Char('p') if modifiers.contains(KeyModifiers::CONTROL) => {
                    state.selected = state.selected.saturating_sub(1);
                }
                KeyCode::Char('n') if modifiers.contains(KeyModifiers::CONTROL) => {
                    state.selected = (state.selected + 1).min(last);
                }
                _ => {
                    if state.query.handle(code, modifiers) {
                        state.query_changed();
                    }
                }
            }
            continue;
        }
        match code {
            KeyCode::Char('c') if modifiers.contains(KeyModifiers::CONTROL) => {
                return Ok(Outcome::Quit);
            }
            KeyCode::Char('/') => {
                state.searching = true;
                state.message = None;
            }
            KeyCode::Char('q') | KeyCode::Esc => return Ok(Outcome::Quit),
            KeyCode::Tab => {
                state.include_remote = !state.include_remote;
                state.query.clear();
                state.selected = 0;
                state.offset = 0;
                state.message = None;
                state.reload()?;
            }
            KeyCode::Up | KeyCode::Char('k') => state.selected = state.selected.saturating_sub(1),
            KeyCode::Down | KeyCode::Char('j') => state.selected = (state.selected + 1).min(last),
            KeyCode::Home | KeyCode::Char('g') => state.selected = 0,
            KeyCode::End | KeyCode::Char('G') => state.selected = last,
            KeyCode::Enter => {
                if let Some(b) = state.selected_branch() {
                    return Ok(Outcome::Switch(b.name.clone(), b.remote.clone()));
                }
            }
            KeyCode::Char(c @ ('d' | 'D')) => {
                if let Some(b) = state.selected_branch() {
                    if b.remote.is_some() {
                        state.message =
                            Some((false, "Remote branches cannot be deleted here".into()));
                        continue;
                    }
                    state.message = Some(git::delete_branch(&b.name, b.is_current, c == 'D')?);
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
    let list_height = state.view.len().min(max_list);

    if state.selected < state.offset {
        state.offset = state.selected;
    } else if state.selected >= state.offset + list_height {
        state.offset = state.selected + 1 - list_height;
    }

    let mut out = stdout();
    queue!(
        out,
        cursor::MoveToColumn(0),
        terminal::Clear(ClearType::FromCursorDown)
    )?;

    let visible = state
        .view
        .iter()
        .enumerate()
        .skip(state.offset)
        .take(list_height);
    for (i, m) in visible {
        let b = &state.branches[m.index];
        let marker = if b.is_current {
            "* "
        } else if b.is_worktree {
            "+ "
        } else {
            "  "
        };
        let text = truncate(&format!("{marker}{}", label(b)), width.saturating_sub(2));
        let (prefix, color) = if i == state.selected {
            ("> ", Some(Color::Cyan))
        } else if b.is_current {
            ("  ", Some(Color::Green))
        } else if b.is_worktree {
            ("  ", Some(Color::Cyan))
        } else if b.remote.is_some() {
            ("  ", Some(Color::DarkGrey))
        } else {
            ("  ", None)
        };
        if let Some(color) = color {
            queue!(out, SetForegroundColor(color))?;
        }
        if i == state.selected {
            queue!(out, SetAttribute(Attribute::Reverse))?;
        }
        queue!(out, Print(prefix))?;
        print_highlighted(&mut out, &text, &m.positions)?;
        queue!(out, SetAttribute(Attribute::Reset), ResetColor)?;
        queue!(out, Print("\r\n"))?;
    }

    if let Some((ok, msg)) = &state.message {
        let color = if *ok { Color::Green } else { Color::Red };
        let msg = msg.lines().next().unwrap_or("");
        queue!(
            out,
            SetForegroundColor(color),
            Print(truncate(msg, width)),
            ResetColor,
            Print("\r\n")
        )?;
    }
    if state.searching {
        let suffix = if state.view.is_empty() {
            "  (no matches)"
        } else {
            ""
        };
        let (before, at, after) = state.query.split_at_cursor();
        queue!(out, Print(format!("/{before}")))?;
        queue!(
            out,
            SetAttribute(Attribute::Reverse),
            Print(at.unwrap_or(' ')),
            SetAttribute(Attribute::Reset),
            Print(format!("{after}{suffix}")),
        )?;
    } else {
        queue!(
            out,
            SetForegroundColor(Color::DarkGrey),
            Print(truncate(hint(state.include_remote), width)),
            ResetColor,
        )?;
    }

    // Return to the top of the region; relative moves stay correct even if printing scrolled.
    let drawn = list_height + footer_lines;
    queue!(out, cursor::MoveToColumn(0))?;
    if drawn > 1 {
        queue!(out, cursor::MoveUp(to_u16(drawn - 1)))?;
    }
    out.flush()?;
    Ok(())
}

const fn hint(include_remote: bool) -> &'static str {
    if include_remote {
        "↑/k ↓/j move · enter switch/track remote · tab hide remote · / search · d delete · D force delete · q quit"
    } else {
        "↑/k ↓/j move · enter switch · tab show remote · / search · d delete · D force delete · q quit"
    }
}

/// Prints `text` (a 2-char marker followed by the label), emphasising the matched label chars.
fn print_highlighted(out: &mut impl Write, text: &str, positions: &[u32]) -> Result<()> {
    for (ci, ch) in text.chars().enumerate() {
        let hit = ci >= 2 && positions.binary_search(&to_u32(ci - 2)).is_ok();
        if hit {
            queue!(
                out,
                SetAttribute(Attribute::Bold),
                SetAttribute(Attribute::Underlined),
                Print(ch),
                SetAttribute(Attribute::NormalIntensity),
                SetAttribute(Attribute::NoUnderline),
            )?;
        } else {
            queue!(out, Print(ch))?;
        }
    }
    Ok(())
}

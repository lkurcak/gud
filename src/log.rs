use crate::git::{self, Commit};
use crate::ui::{to_u16, truncate};
use anyhow::Result;
use crossterm::{
    cursor,
    event::{self, Event, KeyCode, KeyEvent, KeyEventKind, KeyModifiers},
    execute, queue,
    style::{Attribute, Color, Print, ResetColor, SetAttribute, SetForegroundColor},
    terminal::{self, ClearType, EnterAlternateScreen, LeaveAlternateScreen},
};
use std::fmt::Write as _;
use std::fs;
use std::io::{Write, stdout};

/// How many more commits to load each time the selection reaches the end of the list.
const PAGE: usize = 500;
/// Below this terminal width the preview pane is hidden.
const MIN_SPLIT_WIDTH: usize = 80;

/// Runs the UI on the alternate screen and restores the terminal even on early return or panic.
struct TerminalGuard;

impl TerminalGuard {
    fn new() -> Result<Self> {
        terminal::enable_raw_mode()?;
        execute!(stdout(), EnterAlternateScreen, cursor::Hide)?;
        Ok(Self)
    }
}

impl Drop for TerminalGuard {
    fn drop(&mut self) {
        let _ = execute!(stdout(), cursor::Show, LeaveAlternateScreen);
        let _ = terminal::disable_raw_mode();
    }
}

/// Temporarily hands the terminal back, e.g. to an editor or pager.
fn suspended<T>(f: impl FnOnce() -> T) -> Result<T> {
    execute!(stdout(), cursor::Show, LeaveAlternateScreen)?;
    terminal::disable_raw_mode()?;
    let result = f();
    terminal::enable_raw_mode()?;
    execute!(
        stdout(),
        EnterAlternateScreen,
        cursor::Hide,
        terminal::Clear(ClearType::All)
    )?;
    Ok(result)
}

struct Preview {
    hash: String,
    width: usize,
    lines: Vec<String>,
}

struct State {
    commits: Vec<Commit>,
    limit: usize,
    selected: usize,
    offset: usize,
    message: Option<(bool, String)>,
    /// Commit awaiting confirmation of a hard reset.
    confirm_hard_reset: Option<usize>,
    preview: Option<Preview>,
    /// Commit being tagged, the tag name typed so far, and whether the tag is annotated.
    tag_input: Option<(usize, String, bool)>,
}

impl State {
    fn reload(&mut self) -> Result<()> {
        self.commits = git::commits(self.limit)?;
        self.selected = self.selected.min(self.commits.len().saturating_sub(1));
        Ok(())
    }

    fn select(&mut self, index: usize) -> Result<()> {
        // Load more history once the selection hits the end of what is loaded.
        if index + 1 >= self.commits.len() && self.commits.len() == self.limit {
            self.limit += PAGE;
            self.reload()?;
        }
        self.selected = index.min(self.commits.len().saturating_sub(1));
        Ok(())
    }

    fn current(&self) -> Option<&Commit> {
        self.commits.get(self.selected)
    }
}

pub fn run() -> Result<i32> {
    let commits = match git::commits(PAGE) {
        Ok(c) if !c.is_empty() => c,
        _ => {
            eprintln!("No commits.");
            return Ok(1);
        }
    };
    let mut state = State {
        commits,
        limit: PAGE,
        selected: 0,
        offset: 0,
        message: None,
        confirm_hard_reset: None,
        preview: None,
        tag_input: None,
    };

    let _guard = TerminalGuard::new()?;
    event_loop(&mut state)?;
    Ok(0)
}

fn event_loop(state: &mut State) -> Result<()> {
    loop {
        draw(state)?;
        let (code, modifiers) = match event::read()? {
            Event::Key(KeyEvent {
                code,
                modifiers,
                kind: KeyEventKind::Press,
                ..
            }) => (code, modifiers),
            Event::Resize(..) => {
                execute!(stdout(), terminal::Clear(ClearType::All))?;
                continue;
            }
            _ => continue,
        };
        if code == KeyCode::Char('c') && modifiers.contains(KeyModifiers::CONTROL) {
            return Ok(());
        }

        if let Some(index) = state.confirm_hard_reset.take() {
            if matches!(code, KeyCode::Char('y' | 'Y')) {
                reset(state, index, true)?;
            } else {
                state.message = Some((false, "Hard reset cancelled.".to_string()));
            }
            continue;
        }

        if let Some((index, mut name, annotated)) = state.tag_input.take() {
            match code {
                KeyCode::Esc => {
                    state.message = Some((false, "Tag cancelled.".to_string()));
                }
                KeyCode::Enter => tag(state, index, name.trim(), annotated)?,
                KeyCode::Backspace => {
                    name.pop();
                    state.tag_input = Some((index, name, annotated));
                }
                KeyCode::Char(ch) if !modifiers.contains(KeyModifiers::CONTROL) => {
                    name.push(ch);
                    state.tag_input = Some((index, name, annotated));
                }
                _ => state.tag_input = Some((index, name, annotated)),
            }
            continue;
        }

        let page = page_size();
        match code {
            KeyCode::Char('q') | KeyCode::Esc => return Ok(()),
            KeyCode::Up | KeyCode::Char('k') => state.select(state.selected.saturating_sub(1))?,
            KeyCode::Down | KeyCode::Char('j') => state.select(state.selected + 1)?,
            KeyCode::PageUp => state.select(state.selected.saturating_sub(page))?,
            KeyCode::PageDown => state.select(state.selected + page)?,
            KeyCode::Home | KeyCode::Char('g') => state.select(0)?,
            KeyCode::End | KeyCode::Char('G') => state.select(state.commits.len() - 1)?,
            KeyCode::Enter => {
                if let Some(c) = state.current() {
                    let hash = c.hash.clone();
                    suspended(|| git::show(&hash))??;
                }
            }
            KeyCode::Char('e') => edit_message(state)?,
            KeyCode::Char('r') => reset(state, state.selected, false)?,
            KeyCode::Char(key @ ('t' | 'T')) => {
                if state.current().is_some() {
                    state.message = None;
                    state.tag_input = Some((state.selected, String::new(), key == 'T'));
                }
            }
            KeyCode::Char('R') => {
                if let Some(c) = state.current() {
                    state.message = Some((
                        false,
                        format!(
                            "Hard reset to {}? Uncommitted changes will be lost. [y/N]",
                            c.short
                        ),
                    ));
                    state.confirm_hard_reset = Some(state.selected);
                }
            }
            _ => {}
        }
    }
}

fn page_size() -> usize {
    terminal::size().map_or(10, |(_, h)| (h as usize).saturating_sub(3).max(1))
}

fn reset(state: &mut State, index: usize, hard: bool) -> Result<()> {
    let Some(c) = state.commits.get(index) else {
        return Ok(());
    };
    let short = c.short.clone();
    let (ok, output) = git::reset(&c.hash, hard)?;
    let kind = if hard { "Hard" } else { "Soft" };
    state.message = Some(if ok {
        let details = output.lines().next().unwrap_or("");
        let summary = format!("{kind} reset to {short}.");
        (
            true,
            if details.is_empty() {
                summary
            } else {
                format!("{summary} {details}")
            },
        )
    } else {
        (false, output)
    });
    if ok {
        state.selected = 0;
        state.offset = 0;
    }
    state.reload()
}

fn tag(state: &mut State, index: usize, name: &str, annotated: bool) -> Result<()> {
    let Some(c) = state.commits.get(index) else {
        return Ok(());
    };
    if name.is_empty() {
        state.message = Some((false, "Empty tag name; aborted.".to_string()));
        return Ok(());
    }
    let (hash, short) = (c.hash.clone(), c.short.clone());
    let (ok, output) = if annotated {
        let comment = git::comment_char();
        let path = git::git_path("GUD_TAG_EDITMSG")?;
        fs::write(
            &path,
            format!(
                "\n{comment} Write a message for tag {name} on {short}.\n\
                 {comment} Lines starting with '{comment}' will be ignored, \
                 and an empty message aborts.\n"
            ),
        )?;
        let edited = suspended(|| git::edit_file(&path))?;
        let text = fs::read_to_string(&path);
        let _ = fs::remove_file(&path);
        if !edited? {
            state.message = Some((false, "Editor failed; no tag created.".to_string()));
            return Ok(());
        }
        let message = git::stripspace(&text?)?;
        if message.trim().is_empty() {
            state.message = Some((false, "Empty message; aborted.".to_string()));
            return Ok(());
        }
        // Write the cleaned message back so git does not re-interpret comments.
        fs::write(&path, message)?;
        let result = git::tag(name, &hash, Some(&path));
        let _ = fs::remove_file(&path);
        result?
    } else {
        git::tag(name, &hash, None)?
    };
    state.message = Some(if ok {
        let kind = if annotated { "annotated tag" } else { "tag" };
        (true, format!("Created {kind} {name} on {short}."))
    } else {
        (false, output)
    });
    state.reload()
}

fn edit_message(state: &mut State) -> Result<()> {
    let Some(c) = state.current() else {
        return Ok(());
    };
    let (hash, short) = (c.hash.clone(), c.short.clone());
    let original = git::message(&hash)?;
    let comment = git::comment_char();
    let path = git::git_path("GUD_EDITMSG")?;
    fs::write(
        &path,
        format!(
            "{}\n\n{comment} Editing the message of {short}.\n\
             {comment} Lines starting with '{comment}' will be ignored, \
             and an empty message aborts.\n",
            original.trim_end()
        ),
    )?;

    let edited = suspended(|| git::edit_file(&path))?;
    let text = fs::read_to_string(&path);
    let _ = fs::remove_file(&path);
    if !edited? {
        state.message = Some((false, "Editor failed; message unchanged.".to_string()));
        return Ok(());
    }

    let new = git::stripspace(&text?)?;
    state.message = Some(if new.trim().is_empty() {
        (false, "Empty message; aborted.".to_string())
    } else if new == git::stripspace(&original)? {
        (true, "Message unchanged.".to_string())
    } else {
        match git::reword(&hash, &new) {
            Ok((new_hash, count)) => {
                let mut msg = format!("Reworded {short} → {}.", &new_hash[..short.len()]);
                if count > 1 {
                    let _ = write!(msg, " Rewrote {count} commits.");
                }
                (true, msg)
            }
            Err(e) => (false, format!("Reword failed: {e}")),
        }
    });
    state.reload()
}

fn draw(state: &mut State) -> Result<()> {
    let (width, height) = match terminal::size()? {
        (0, _) | (_, 0) => (80, 24),
        (w, h) => (w as usize, h as usize),
    };
    let footer_lines = if state.message.is_some() || state.tag_input.is_some() {
        2
    } else {
        1
    };
    let body_height = height.saturating_sub(footer_lines).max(1);
    let list_height = state.commits.len().min(body_height);

    if state.selected < state.offset {
        state.offset = state.selected;
    } else if state.selected >= state.offset + list_height {
        state.offset = state.selected + 1 - list_height;
    }

    let (list_width, preview_width) = if width >= MIN_SPLIT_WIDTH {
        let list = width / 2;
        (list, width - list - 2)
    } else {
        (width, 0)
    };
    if preview_width > 0 {
        update_preview(state, preview_width);
    }

    let mut out = stdout();
    for row in 0..body_height {
        queue!(out, cursor::MoveTo(0, to_u16(row)))?;
        let index = state.offset + row;
        if let Some(c) = state.commits.get(index) {
            draw_commit(&mut out, c, index == state.selected, list_width)?;
        } else {
            queue!(out, Print(" ".repeat(list_width)))?;
        }
        if preview_width > 0 {
            queue!(
                out,
                SetForegroundColor(Color::DarkGrey),
                Print("│ "),
                ResetColor
            )?;
            let line = state
                .preview
                .as_ref()
                .and_then(|p| p.lines.get(row))
                .map_or("", String::as_str);
            let color = if row == 0 { Some(Color::Yellow) } else { None };
            print_span(&mut out, color, line, preview_width)?;
        }
        queue!(out, terminal::Clear(ClearType::UntilNewLine))?;
    }

    let mut row = body_height;
    if let Some((_, name, annotated)) = &state.tag_input {
        let kind = if *annotated { "Annotated tag" } else { "Tag" };
        queue!(out, cursor::MoveTo(0, to_u16(row)))?;
        print_span(
            &mut out,
            Some(Color::Yellow),
            &format!("{kind} name: {name}█  (enter to continue, esc to cancel)"),
            width,
        )?;
        queue!(out, terminal::Clear(ClearType::UntilNewLine))?;
        row += 1;
    } else if let Some((ok, msg)) = &state.message {
        let color = if state.confirm_hard_reset.is_some() {
            Color::Yellow
        } else if *ok {
            Color::Green
        } else {
            Color::Red
        };
        let msg = msg.lines().next().unwrap_or("");
        queue!(out, cursor::MoveTo(0, to_u16(row)))?;
        print_span(&mut out, Some(color), msg, width)?;
        queue!(out, terminal::Clear(ClearType::UntilNewLine))?;
        row += 1;
    }
    queue!(out, cursor::MoveTo(0, to_u16(row)))?;
    print_span(
        &mut out,
        Some(Color::DarkGrey),
        "↑/k ↓/j move · enter show · e edit message · t/T tag/annotated tag · r soft reset · R hard reset · q quit",
        width,
    )?;
    queue!(out, terminal::Clear(ClearType::UntilNewLine))?;
    out.flush()?;
    Ok(())
}

fn update_preview(state: &mut State, width: usize) {
    let Some(c) = state.current() else {
        state.preview = None;
        return;
    };
    if state
        .preview
        .as_ref()
        .is_some_and(|p| p.hash == c.hash && p.width == width)
    {
        return;
    }
    let text = git::preview(&c.hash, width).unwrap_or_else(|e| e.to_string());
    state.preview = Some(Preview {
        hash: c.hash.clone(),
        width,
        lines: text
            .lines()
            .map(|l| l.replace('\t', "    ").replace('\r', ""))
            .collect(),
    });
}

fn draw_commit(out: &mut impl Write, c: &Commit, selected: bool, width: usize) -> Result<()> {
    let refs = if c.refs.is_empty() {
        String::new()
    } else {
        format!("({}) ", c.refs)
    };
    if selected {
        let line = format!("> {} {refs}{}", c.short, c.subject);
        queue!(
            out,
            SetForegroundColor(Color::Cyan),
            SetAttribute(Attribute::Reverse),
        )?;
        print_span(out, None, &line, width)?;
        queue!(out, SetAttribute(Attribute::Reset), ResetColor)?;
        return Ok(());
    }
    let spans = [
        (None, "  "),
        (Some(Color::Yellow), c.short.as_str()),
        (None, " "),
        (Some(Color::Green), refs.as_str()),
        (None, c.subject.as_str()),
    ];
    let mut left = width;
    for (color, text) in spans {
        let text = truncate(text, left);
        left -= text.chars().count();
        if let Some(color) = color {
            queue!(out, SetForegroundColor(color), Print(text), ResetColor)?;
        } else {
            queue!(out, Print(text))?;
        }
    }
    queue!(out, Print(" ".repeat(left)))?;
    Ok(())
}

/// Prints `text` clipped and padded to exactly `width` columns.
fn print_span(out: &mut impl Write, color: Option<Color>, text: &str, width: usize) -> Result<()> {
    let text = truncate(text, width);
    let pad = width - text.chars().count();
    if let Some(color) = color {
        queue!(out, SetForegroundColor(color), Print(text), ResetColor)?;
    } else {
        queue!(out, Print(text))?;
    }
    queue!(out, Print(" ".repeat(pad)))?;
    Ok(())
}

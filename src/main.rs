// Declares the module; its code lives in src/events.rs.
mod events;

use std::error::Error;
use std::io::Write;
use std::{env, io};

use ratatui::crossterm::event::{self, KeyCode, KeyEventKind};
use ratatui::layout::{Constraint, Layout};
use chrono::Local;
use ratatui::style::{Color, Style, Stylize};
use ratatui::text::{Line, Span, Text};
use ratatui::widgets::{Block, List, ListState, Paragraph, Wrap};
use ratatui::{DefaultTerminal, Frame};

use events::{Event, load_events};

// All mutable state of the UI lives here; drawing only reads it.
struct App {
    events: Vec<Event>,
    selected: usize,
}

impl App {
    // The main loop: draw, wait for a key, update state, repeat until quit.
    fn run(&mut self, terminal: &mut DefaultTerminal) -> io::Result<()> {
        loop {
            terminal.draw(|frame| self.draw(frame))?;
            // read() blocks until the terminal sends something.
            let event::Event::Key(key) = event::read()? else { continue };
            // Some terminals also report key releases; only react to presses.
            if key.kind != KeyEventKind::Press {
                continue;
            }
            match key.code {
                KeyCode::Char('q') | KeyCode::Esc => return Ok(()),
                KeyCode::Down | KeyCode::Char('j') => self.select_next(),
                KeyCode::Up | KeyCode::Char('k') => self.select_previous(),
                _ => {}
            }
        }
    }

    fn select_next(&mut self) {
        if self.selected + 1 < self.events.len() {
            self.selected += 1;
        }
    }

    fn select_previous(&mut self) {
        // saturating_sub stops at 0 instead of underflowing an unsigned integer.
        self.selected = self.selected.saturating_sub(1);
    }

    // Immediate mode: the whole screen is rebuilt from state on every draw.
    fn draw(&self, frame: &mut Frame) {
        let [list_area, detail_area] =
            Layout::horizontal([Constraint::Percentage(40), Constraint::Percentage(60)])
                .areas(frame.area());

        let today = Local::now().date_naive();
        let items = self.events.iter().map(|event| {
            let day = event.start.map_or("          ".to_string(), |t| t.format("%Y-%m-%d").to_string());
            // Span = a run of text with one style; a Line is a Vec of Spans.
            let date_style = match event.start.map(|t| t.date_naive()) {
                Some(d) if d < today => Style::new().dark_gray(),
                Some(d) if d == today => Style::new().yellow().bold(),
                _ => Style::new().cyan(),
            };
            let summary_style = if event.all_day { Style::new().magenta() } else { Style::new() };
            Line::from(vec![
                Span::styled(day, date_style),
                Span::raw("  "),
                Span::styled(event.summary.as_str(), summary_style),
            ])
        });
        let list = List::new(items)
            .block(Block::bordered().title(" Events ".bold()).title_bottom(" q quit  j/k move ".dark_gray()).border_style(Color::Blue))
            .highlight_style(Style::new().bg(Color::Blue).white().bold())
            .highlight_symbol("▶ ");
        let mut state = ListState::default().with_selected(Some(self.selected));
        frame.render_stateful_widget(list, list_area, &mut state);

        let detail = match self.events.get(self.selected) {
            Some(event) => event_details(event),
            None => Text::from("No events in this file."),
        };
        let paragraph = Paragraph::new(detail)
            .block(Block::bordered().title(" Details ".bold()).border_style(Color::Green))
            .wrap(Wrap { trim: false });
        frame.render_widget(paragraph, detail_area);
    }
}

fn event_details(event: &Event) -> Text<'_> {
    // Closure capturing nothing: a small helper to build one "Label: value" line.
    let field = |label: &'static str, value: String| {
        Line::from(vec![Span::styled(label, Style::new().green()), Span::raw(value)])
    };
    let mut lines = vec![
        Line::from(event.summary.as_str().yellow().bold()),
        field("Start:    ", event.format_time(event.start)),
        field("End:      ", event.format_time(event.end)),
        field("Location: ", event.location.as_deref().unwrap_or("(none)").to_string()),
        Line::from(""),
    ];
    if let Some(description) = &event.description {
        lines.extend(description.lines().map(Line::from));
    }
    Text::from(lines)
}

// const needs an explicit type and is inlined wherever it is used.
const USAGE: &str = "usage: mutt-ics-reader [--plain] <file.ics>";

// &[Event] is a slice: a read-only view that a &Vec<Event> converts to automatically.
// println! panics if stdout is closed (e.g. piped into `head`); writeln! returns the error instead.
fn print_plain(events: &[Event]) -> io::Result<()> {
    let mut out = io::stdout().lock();
    // Colors are raw ANSI escape codes here, since there is no terminal to draw on.
    // NO_COLOR is the convention for disabling them: https://no-color.org
    let color = env::var_os("NO_COLOR").is_none();
    // This closure captures `color` from the surrounding scope.
    let paint = |code: &str, text: &str| {
        if color { format!("\x1b[{code}m{text}\x1b[0m") } else { text.to_string() }
    };
    for event in events {
        writeln!(out, "{}", paint("1;33", &event.summary))?;
        writeln!(out, "  {}{}", paint("32", "Start:    "), event.format_time(event.start))?;
        writeln!(out, "  {}{}", paint("32", "End:      "), event.format_time(event.end))?;
        writeln!(out, "  {}{}", paint("32", "Location: "), event.location.as_deref().unwrap_or("(none)"))?;
        if let Some(description) = &event.description {
            for line in description.lines() {
                writeln!(out, "  {line}")?;
            }
        }
        writeln!(out)?;
    }
    Ok(())
}

fn main() -> Result<(), Box<dyn Error>> {
    let mut plain = false;
    // Type inferred later from `path = Some(arg)`.
    let mut path = None;
    // skip(1) drops the program name, which is always the first argument.
    for arg in env::args().skip(1) {
        // as_str() turns the String into &str so it can be matched against literals.
        match arg.as_str() {
            "--plain" => plain = true,
            // A guard: first positional argument is the path, a second one is an error.
            _ if path.is_none() => path = Some(arg),
            // into() converts &str to the Box<dyn Error> declared in the return type.
            _ => return Err(USAGE.into()),
        }
    }
    let path = path.ok_or(USAGE)?;
    let events = load_events(&path)?;

    if plain {
        return match print_plain(&events) {
            // The reader went away (`| head`): not an error worth reporting.
            Err(e) if e.kind() == io::ErrorKind::BrokenPipe => Ok(()),
            result => Ok(result?),
        };
    }

    // init() switches the terminal to raw + alternate screen; restore() must undo it
    // even if run() failed, otherwise the shell is left in a broken state.
    let mut terminal = ratatui::init();
    let result = App { events, selected: 0 }.run(&mut terminal);
    ratatui::restore();
    Ok(result?)
}

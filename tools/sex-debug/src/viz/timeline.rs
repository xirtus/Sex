use ratatui::{
    backend::CrosstermBackend,
    layout::{Constraint, Direction, Layout},
    style::{Color, Style},
    text::{Line, Span},
    widgets::{Block, Borders, Paragraph},
    Terminal,
};
use std::io;
use crossterm::{
    event::{self, Event, KeyCode},
    execute,
    terminal::{disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen},
};
use crate::model::TraceEvent;
use crate::domain::Domain;

pub fn run_tui(events: &[TraceEvent], highlight_index: Option<usize>) -> io::Result<()> {
    enable_raw_mode()?;
    let mut stdout = io::stdout();
    execute!(stdout, EnterAlternateScreen)?;
    let backend = CrosstermBackend::new(stdout);
    let mut terminal = Terminal::new(backend)?;

    let mut offset = highlight_index.unwrap_or(0).saturating_sub(5); // Scroll to show highlight with some context

    loop {
        terminal.draw(|f| {
            let chunks = Layout::default()
                .direction(Direction::Vertical)
                .margin(1)
                .constraints([Constraint::Min(3), Constraint::Length(3)].as_ref())
                .split(f.area());

            let mut lines = Vec::new();
            for (i, event) in events.iter().enumerate().skip(offset).take(f.area().height as usize - 4) {
                let color = match event.domain {
                    Domain::Kernel => Color::Red,
                    Domain::Linen => Color::Green,
                    Domain::SexDisplay => Color::Blue,
                    Domain::Unknown(_) => Color::DarkGray,
                };

                let domain_str = format!("{:?}", event.domain);
                let sym = event.symbol.as_deref().unwrap_or("??");

                let mut style = Style::default();
                if Some(i) == highlight_index {
                    style = style.bg(Color::DarkGray).fg(Color::White); // Highlight background
                }

                let line = Line::from(vec![
                    Span::styled(format!("{:<15} ", event.tsc), style.fg(Color::Yellow)),
                    Span::styled(format!("[ {:<12} ] ", domain_str), style.fg(color)),
                    Span::styled(format!("PKRU: 0x{:08X} ", event.pkru), style),
                    Span::styled(sym.to_string(), style.fg(Color::Cyan)),
                ]);
                lines.push(line);
            }

            let p = Paragraph::new(lines)
                .block(Block::default().borders(Borders::ALL).title("Timeline"));
            f.render_widget(p, chunks[0]);

            let help = Paragraph::new("Press 'q' to quit, Up/Down to scroll")
                .block(Block::default().borders(Borders::ALL).title("Help"));
            f.render_widget(help, chunks[1]);
        })?;

        if let Event::Key(key) = event::read()? {
            match key.code {
                KeyCode::Char('q') => break,
                KeyCode::Down => {
                    if offset + 1 < events.len() {
                        offset += 1;
                    }
                }
                KeyCode::Up => {
                    if offset > 0 {
                        offset -= 1;
                    }
                }
                _ => {}
            }
        }
    }

    disable_raw_mode()?;
    execute!(terminal.backend_mut(), LeaveAlternateScreen)?;
    Ok(())
}
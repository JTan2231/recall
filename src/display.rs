use crossterm::{
    event::{self, DisableMouseCapture, EnableMouseCapture, Event, KeyCode},
    execute,
    terminal::{disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen},
};
use ratatui::{
    backend::{Backend, CrosstermBackend},
    layout::{Constraint, Direction, Layout},
    style::{Color, Modifier, Style},
    text::Span,
    widgets::{Block, Borders, List, ListItem, ListState, Paragraph},
    Terminal,
};
use std::{error::Error, io};

use crate::files;

struct StatefulList<T> {
    state: ListState,
    items: Vec<T>,
}

impl<T> StatefulList<T> {
    fn with_items(items: Vec<T>) -> StatefulList<T> {
        StatefulList {
            state: ListState::default(),
            items,
        }
    }

    fn next(&mut self) {
        let i = match self.state.selected() {
            Some(i) => {
                if i >= self.items.len() - 1 {
                    0
                } else {
                    i + 1
                }
            }
            None => 0,
        };
        self.state.select(Some(i));
    }

    fn previous(&mut self) {
        let i = match self.state.selected() {
            Some(i) => {
                if i == 0 {
                    self.items.len() - 1
                } else {
                    i - 1
                }
            }
            None => 0,
        };
        self.state.select(Some(i));
    }

    fn selected_item(&self) -> Option<&T> {
        self.state.selected().map(|i| &self.items[i])
    }
}

pub fn terminal_testing() -> Result<(), Box<dyn Error>> {
    enable_raw_mode()?;
    let mut stdout = io::stdout();
    execute!(stdout, EnterAlternateScreen, EnableMouseCapture)?;
    let backend = CrosstermBackend::new(stdout);
    let mut terminal = Terminal::new(backend)?;
    let items = files::get_tracked_files();
    let mut content_map = std::collections::HashMap::new();
    for file in items.iter() {
        let content = std::fs::read_to_string(file).unwrap();
        content_map.insert(file.clone(), content);
    }

    let mut list = StatefulList::with_items(items);
    let res = run_app(&mut terminal, &mut list, &content_map);
    disable_raw_mode()?;
    execute!(
        terminal.backend_mut(),
        LeaveAlternateScreen,
        DisableMouseCapture
    )?;
    terminal.show_cursor()?;
    if let Err(err) = res {
        println!("{:?}", err)
    }

    Ok(())
}

fn run_app<B: Backend>(
    terminal: &mut Terminal<B>,
    list: &mut StatefulList<String>,
    content_map: &std::collections::HashMap<String, String>,
) -> io::Result<()> {
    loop {
        terminal.draw(|f| {
            let size = f.size();
            let chunks = Layout::default()
                .direction(Direction::Horizontal)
                .constraints([Constraint::Percentage(50), Constraint::Percentage(50)].as_ref())
                .split(size);
            let items: Vec<ListItem> = list
                .items
                .iter()
                .map(|i| ListItem::new(Span::raw(i)))
                .collect();
            let list_widget = List::new(items)
                .block(Block::default().borders(Borders::ALL).title(" List "))
                .highlight_style(
                    Style::default()
                        .bg(Color::LightGreen)
                        .add_modifier(Modifier::BOLD),
                )
                .highlight_symbol(">> ");
            f.render_stateful_widget(list_widget, chunks[0], &mut list.state);

            let default = String::new();
            let selected_item = list.selected_item().unwrap_or(&default);

            // if the selected item is an existing tracked file
            let details =
                Paragraph::new(content_map.get(selected_item).unwrap_or(&default).as_str())
                    .block(Block::default().borders(Borders::ALL).title(" Details "));
            f.render_widget(details, chunks[1]);
        })?;
        if let Event::Key(key) = event::read()? {
            match key.code {
                KeyCode::Char('q') => return Ok(()),
                KeyCode::Down => list.next(),
                KeyCode::Up => list.previous(),
                _ => {}
            }
        }
    }
}

use crossterm::{
    event::{self, DisableMouseCapture, EnableMouseCapture, Event, KeyCode},
    execute,
    terminal::{disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen},
};
use std::{
    io::{self, Stdout},
    thread,
    time::Duration,
};
use tracing::instrument;
use tui::{
    backend::CrosstermBackend,
    layout::{Constraint, Direction, Layout},
    style::{Color, Modifier, Style},
    text::{Span, Spans},
    widgets::{Block, Borders, Cell, Row, Table, Widget},
    Terminal,
};

use psemu_core::{Cpu, REGISTER_NAMES};

pub struct Debugger {
    cpu: Cpu,
}

impl Debugger {
    pub fn new() -> Self {
        Debugger { cpu: Cpu::new() }
    }

    pub fn run(&mut self) {
        let mut term = setup_terminal().unwrap();
        self.display(&mut term);
        thread::sleep(Duration::from_millis(5000));
        self.cpu.run_single_cycle();
        self.display(&mut term);
        thread::sleep(Duration::from_millis(5000));
        restore_terminal(&mut term);
    }

    pub fn display(
        &self,
        terminal: &mut Terminal<CrosstermBackend<Stdout>>,
    ) -> Result<(), io::Error> {
        let mut rows = Vec::new();
        for (i, reg) in self.cpu.get_registers().iter().enumerate() {
            rows.push(Row::new(vec![
                format!("{i}"),
                REGISTER_NAMES[i].to_string(),
                format!("{reg:#x}").to_string(),
            ]));
        }

        let table = Table::new(rows)
            // You can set the style of the entire Table.
            .style(Style::default().fg(Color::White))
            // It has an optional header, which is simply a Row always visible at the top.
            .header(
                Row::new(vec!["#", "Name", "Value"])
                    .style(Style::default().fg(Color::Yellow))
                    // If you want some space between the header and the rest of the rows, you can always
                    // specify some margin at the bottom.
                    .bottom_margin(1),
            )
            // As any other widget, a Table can be wrapped in a Block.
            .block(
                Block::default()
                    .title("psemudb - registers")
                    .borders(Borders::ALL),
            )
            // Columns widths are constrained in the same way as Layout...
            .widths(&[
                Constraint::Length(2),
                Constraint::Length(5),
                Constraint::Length(18),
            ])
            // ...and they can be separated by a fixed spacing.
            .column_spacing(1)
            // If you wish to highlight a row in any specific way when it is selected...
            .highlight_style(Style::default().add_modifier(Modifier::BOLD))
            // ...and potentially show a symbol in front of the selection.
            .highlight_symbol(">>");

        terminal.draw(|f| {
            let size = f.size();
            f.render_widget(table, size);
        })?;

        Ok(())
    }
}

pub fn setup_terminal() -> Result<Terminal<CrosstermBackend<Stdout>>, io::Error> {
    enable_raw_mode()?;
    let mut stdout = io::stdout();
    execute!(stdout, EnterAlternateScreen, EnableMouseCapture)?;
    let backend = CrosstermBackend::new(stdout);
    let terminal = Terminal::new(backend)?;
    Ok(terminal)
}

pub fn restore_terminal(
    terminal: &mut Terminal<CrosstermBackend<Stdout>>,
) -> Result<(), io::Error> {
    disable_raw_mode()?;
    execute!(
        terminal.backend_mut(),
        LeaveAlternateScreen,
        DisableMouseCapture
    )?;
    terminal.show_cursor()?;

    Ok(())
}

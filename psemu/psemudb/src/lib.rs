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
use tui::{
    backend::CrosstermBackend,
    layout::{Constraint, Direction, Layout},
    style::{Color, Modifier, Style},
    text::{Span, Spans},
    widgets::{Block, Borders, Cell, Row, Table, Widget, Tabs},
    Terminal,
};

use psemu_core::{Cpu, REGISTER_NAMES};

#[derive(Copy, Clone, Debug)]
enum MenuItem {
    Home,
    NextInstruction,
    Quit,
}

impl From<MenuItem> for usize {
    fn from(input: MenuItem) -> usize {
        match input {
            MenuItem::Home => 0,
            MenuItem::NextInstruction => 1,
            MenuItem::Quit => 2,
        }
    }
}

pub struct Debugger {
    cpu: Cpu,
    prev_registers: [u32; 32],
}

impl Debugger {
    pub fn new() -> Self {
        let cpu = Cpu::new();
        let prev_registers: [u32; 32] = cpu.get_registers().try_into().unwrap();

        Debugger { cpu, prev_registers }
    }

    pub fn run(&mut self) {
        let mut term = setup_terminal().unwrap();

        self.display(&mut term);
        thread::sleep(Duration::from_millis(5000));

        let tmp: [u32; 32] = self.cpu.get_registers().try_into().unwrap();
        self.cpu.run_single_cycle();
        self.prev_registers = tmp;

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
            let mut row = Row::new(vec![
                format!("{i}"),
                REGISTER_NAMES[i].to_string(),
                format!("{reg:#x}").to_string(),
            ]);
            if *reg != self.prev_registers[i] {
                row = row.style(Style::default().fg(Color::LightRed));
            }
            rows.push(row);
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
                    // .bottom_margin(1),
            )
            // As any other widget, a Table can be wrapped in a Block.
            .block(
                Block::default()
                    .title("registers")
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

        let asm = Block::default()
            .title("asm")
            .borders(Borders::ALL);

        let menu_titles = vec!["Home", "Next Instruction", "Quit"];
        let mut active_menu_item = MenuItem::Home;

        terminal.draw(|f| {

            let size = f.size();
            let chunks = Layout::default()
                .direction(Direction::Vertical)
                // .margin(2)
                .constraints(
                    [
                        Constraint::Length(3),
                        Constraint::Min(2),
                        // Constraint::Length(3),
                    ]
                    .as_ref(),
                )
                .split(size);

            let menu = menu_titles
            .iter()
            .map(|t| {
                let (first, rest) = t.split_at(1);
                Spans::from(vec![
                    Span::styled(
                        first,
                        Style::default()
                            .fg(Color::Yellow)
                            .add_modifier(Modifier::UNDERLINED),
                    ),
                    Span::styled(rest, Style::default().fg(Color::White)),
                ])
            })
            .collect();

            let tabs = Tabs::new(menu)
                .select(active_menu_item.into())
                .block(Block::default().title("Menu").borders(Borders::ALL))
                .style(Style::default().fg(Color::White))
                .highlight_style(Style::default().fg(Color::Yellow))
                .divider(Span::raw("|"));


            // let size = f.size();
            // f.render_widget(table, size);
            let chunks2 = Layout::default()
                .direction(Direction::Horizontal)
                // .margin(1)
                .constraints(
                    [
                        Constraint::Percentage(20),
                        Constraint::Percentage(80)
                    ].as_ref()
                )
                .split(chunks[1]);
            f.render_widget(table, chunks2[0]);
            f.render_widget(asm, chunks2[1]);


            f.render_widget(tabs, chunks[0]);
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

use ansi_to_tui::IntoText;
use crossterm::{
    event::{
        self, DisableMouseCapture, EnableMouseCapture, Event, KeyCode, KeyEvent, KeyEventKind,
    },
    execute,
    terminal::{disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen},
};
use std::{
    io::{self, Stdout},
    sync::{Arc, Mutex},
};
use tui::{
    backend::CrosstermBackend,
    layout::{Constraint, Direction, Layout},
    style::{Color, Modifier, Style},
    text::{Span, Spans},
    widgets::{Block, Borders, Row, Table, Tabs},
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
    logs: Arc<Mutex<Vec<String>>>,
}

impl Debugger {
    pub fn new(logs: Arc<Mutex<Vec<String>>>) -> Self {
        let cpu = Cpu::new();
        let prev_registers: [u32; 32] = cpu.get_registers().try_into().unwrap();

        Debugger {
            cpu,
            prev_registers,
            logs,
        }
    }

    pub fn run(&mut self) {
        let mut term = setup_terminal().unwrap();

        self.display(&mut term);
        // thread::sleep(Duration::from_millis(5000));

        loop {
            let event = listen_to_events();
            if let Ok(KeyEvent { code, kind, .. }) = event {
                if code == KeyCode::Char('q') && kind == KeyEventKind::Press {
                    restore_terminal(&mut term);
                    break;
                } else if code == KeyCode::Char('n') && kind == KeyEventKind::Press {
                    let tmp: [u32; 32] = self.cpu.get_registers().try_into().unwrap();
                    self.cpu.run_single_cycle();
                    self.prev_registers = tmp;
                    self.display(&mut term);
                }
            }
        }
        std::process::exit(0);
    }

    fn get_registers_table(&self) -> Table {
        let mut rows = Vec::new();
        for (i, reg) in self.cpu.get_registers().iter().enumerate() {
            let mut row = Row::new(vec![
                format!("{i}"),
                REGISTER_NAMES[i].to_string(),
                format!("{reg:#010x}").to_string(),
            ]);
            if *reg != self.prev_registers[i] {
                row = row.style(Style::default().fg(Color::LightRed));
            }
            rows.push(row);
        }

        Table::new(rows)
            // You can set the style of the entire Table.
            .style(Style::default().fg(Color::White))
            // It has an optional header, which is simply a Row always visible at the top.
            .header(
                Row::new(vec!["#", "Name", "Value"]).style(Style::default().fg(Color::Yellow)), // If you want some space between the header and the rest of the rows, you can always
                                                                                                // specify some margin at the bottom.
                                                                                                // .bottom_margin(1),
            )
            // As any other widget, a Table can be wrapped in a Block.
            .block(Block::default().title("registers").borders(Borders::ALL))
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
            .highlight_symbol(">>")
    }

    fn get_asm_instructions_table(&self) -> Table {
        let mut rows = Vec::new();
        for instr in &self.cpu.instruction_history {
            let row = Row::new(vec![
                format!("{:#010x}", instr.raw),
                instr.op.to_owned(),
                instr.human.0.to_owned(),
                instr.eval.0.to_owned(),
            ]);
            rows.push(row);
        }

        Table::new(rows)
            // You can set the style of the entire Table.
            .style(Style::default().fg(Color::White))
            // It has an optional header, which is simply a Row always visible at the top.
            .header(
                Row::new(vec!["raw", "op", "human", "evaluated"])
                    .style(Style::default().fg(Color::Yellow)), // If you want some space between the header and the rest of the rows, you can always
                                                                // specify some margin at the bottom.
                                                                // .bottom_margin(1),
            )
            // As any other widget, a Table can be wrapped in a Block.
            .block(
                Block::default()
                    .title("asm instructions")
                    .borders(Borders::ALL),
            )
            // Columns widths are constrained in the same way as Layout...
            .widths(&[
                Constraint::Length(10),
                Constraint::Length(5),
                Constraint::Percentage(30),
                Constraint::Percentage(70),
            ])
            // ...and they can be separated by a fixed spacing.
            .column_spacing(1)
            // If you wish to highlight a row in any specific way when it is selected...
            .highlight_style(Style::default().add_modifier(Modifier::BOLD))
            // ...and potentially show a symbol in front of the selection.
            .highlight_symbol(">>")
    }

    // TODO: Extract this + ChannelLogger into separate crate and publish on crates.io
    fn get_logs_table(&self) -> Table {
        let mut rows = Vec::new();
        if let Ok(logs) = &self.logs.lock() {
            for log in logs.iter() {
                // For some reason the colors are duller when using this than stdout
                // Maybe has to do with the bold vs normal font weight?
                // This prints as normal, but stdout uses bold for some of the text
                let s = log.into_text().unwrap();
                let row = Row::new(vec![s]);
                rows.push(row);
            }
        }

        Table::new(rows)
            // You can set the style of the entire Table.
            .style(Style::default().fg(Color::White))
            // It has an optional header, which is simply a Row always visible at the top.
            // .header(
            //     Row::new(vec!["raw", "op", "human", "evaluated"])
            //         .style(Style::default().fg(Color::Yellow)), // If you want some space between the header and the rest of the rows, you can always
            //                                                     // specify some margin at the bottom.
            //                                                     // .bottom_margin(1),
            // )
            // As any other widget, a Table can be wrapped in a Block.
            .block(Block::default().title("logs").borders(Borders::ALL))
            // Columns widths are constrained in the same way as Layout...
            .widths(&[Constraint::Percentage(100)])
            // ...and they can be separated by a fixed spacing.
            .column_spacing(1)
            // If you wish to highlight a row in any specific way when it is selected...
            .highlight_style(Style::default().add_modifier(Modifier::BOLD))
            // ...and potentially show a symbol in front of the selection.
            .highlight_symbol(">>")
    }

    pub fn display(
        &self,
        terminal: &mut Terminal<CrosstermBackend<Stdout>>,
    ) -> Result<(), io::Error> {
        let registers_table = self.get_registers_table();

        let asm_instructions_table = self.get_asm_instructions_table();
        let logs_table = self.get_logs_table();

        let menu_titles = vec!["Home", "Next Instruction", "Quit"];
        let mut active_menu_item = MenuItem::Home;

        terminal.draw(|f| {
            let size = f.size();
            let outer_view_chunks = Layout::default()
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

            let main_view_chunks = Layout::default()
                .direction(Direction::Horizontal)
                // .margin(1)
                .constraints([Constraint::Percentage(15), Constraint::Percentage(85)].as_ref())
                .split(outer_view_chunks[1]);
            f.render_widget(registers_table, main_view_chunks[0]);

            let right_subview_chunks = Layout::default()
                .direction(Direction::Vertical)
                // .margin(1)
                .constraints([Constraint::Percentage(80), Constraint::Percentage(20)].as_ref())
                .split(main_view_chunks[1]);
            f.render_widget(asm_instructions_table, right_subview_chunks[0]);
            f.render_widget(logs_table, right_subview_chunks[1]);

            f.render_widget(tabs, outer_view_chunks[0]);
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

fn listen_to_events() -> crossterm::Result<KeyEvent> {
    loop {
        // `read()` blocks until an `Event` is available
        match event::read()? {
            // Event::FocusGained => println!("FocusGained"),
            // Event::FocusLost => println!("FocusLost"),
            Event::Key(event) => {
                // println!("{:?}", event);
                return Ok(event);
            }
            // Event::Mouse(event) => println!("{:?}", event),
            // #[cfg(feature = "bracketed-paste")]
            // Event::Paste(data) => println!("{:?}", data),
            // Event::Resize(width, height) => println!("New size {}x{}", width, height),
            _ => (),
        }
    }
}

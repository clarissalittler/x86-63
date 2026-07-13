use std::collections::BTreeSet;
use std::io::{self, Stdout};

use crossterm::cursor::Show;
use crossterm::event::{self, Event, KeyCode, KeyEventKind};
use crossterm::execute;
use crossterm::terminal::{
    EnterAlternateScreen, LeaveAlternateScreen, disable_raw_mode, enable_raw_mode,
};
use ratatui::Terminal;
use ratatui::backend::CrosstermBackend;
use ratatui::layout::{Constraint, Direction, Layout, Rect};
use ratatui::style::{Color, Modifier, Style};
use ratatui::text::{Line, Span, Text};
use ratatui::widgets::{Block, Borders, Paragraph, Wrap};
use x86_63_core::{
    Command, CommandResult, MachineStatus, MachineView, ProgramView, Session, StepEvent,
};

pub fn run(session: Session) -> io::Result<()> {
    let _terminal_session = TerminalSession::enter()?;
    let stdout = io::stdout();
    let backend = CrosstermBackend::new(stdout);
    let mut terminal = Terminal::new(backend)?;
    run_loop(&mut terminal, App::new(session))
}

struct TerminalSession;

impl TerminalSession {
    fn enter() -> io::Result<Self> {
        enable_raw_mode()?;
        let mut stdout = io::stdout();
        if let Err(error) = execute!(stdout, EnterAlternateScreen) {
            let _ = disable_raw_mode();
            return Err(error);
        }
        Ok(Self)
    }
}

impl Drop for TerminalSession {
    fn drop(&mut self) {
        let _ = disable_raw_mode();
        let _ = execute!(io::stdout(), LeaveAlternateScreen, Show);
    }
}

fn run_loop(terminal: &mut Terminal<CrosstermBackend<Stdout>>, mut app: App) -> io::Result<()> {
    loop {
        terminal.draw(|frame| draw(frame, &app))?;
        if let Event::Key(key) = event::read()?
            && key.kind == KeyEventKind::Press
        {
            let command = match key.code {
                KeyCode::Char('q') | KeyCode::Esc => return Ok(()),
                KeyCode::Char('m') => {
                    app.detail = app.detail.toggle();
                    None
                }
                KeyCode::Char('s') | KeyCode::Right | KeyCode::Enter => Some(Command::Step),
                KeyCode::Char('n') => Some(Command::Next),
                KeyCode::Char('b') | KeyCode::Left => Some(Command::Back),
                KeyCode::Char('r') => Some(Command::Reset),
                KeyCode::Char('c') => Some(Command::Continue { max_steps: 10_000 }),
                _ => None,
            };
            if let Some(command) = command {
                let result = app.session.execute(command);
                if result.events.iter().any(|event| {
                    matches!(
                        event,
                        StepEvent::EffectiveAddress { .. }
                            | StepEvent::MemoryRead { .. }
                            | StepEvent::MemoryWrite { .. }
                            | StepEvent::Output { .. }
                    )
                }) {
                    app.detail = DetailPane::Memory;
                }
                app.last = Some(result);
            }
        }
    }
}

struct App {
    session: Session,
    program: ProgramView,
    last: Option<CommandResult>,
    detail: DetailPane,
}

impl App {
    fn new(session: Session) -> Self {
        let program = session.program();
        Self {
            session,
            program,
            last: None,
            detail: DetailPane::Registers,
        }
    }

    fn view(&self) -> MachineView {
        self.last
            .as_ref()
            .map_or_else(|| self.session.view(), |result| result.view.clone())
    }

    fn explanation(&self) -> &str {
        self.last.as_ref().map_or_else(
            || self.session.last_explanation(),
            |result| &result.explanation,
        )
    }

    fn changed_registers(&self) -> BTreeSet<String> {
        self.last
            .as_ref()
            .into_iter()
            .flat_map(|result| &result.events)
            .filter_map(|event| match event {
                StepEvent::RegisterWrite { canonical, .. } => Some(canonical.clone()),
                _ => None,
            })
            .collect()
    }

    fn changed_memory(&self) -> BTreeSet<String> {
        self.last
            .as_ref()
            .into_iter()
            .flat_map(|result| &result.events)
            .filter_map(|event| match event {
                StepEvent::MemoryWrite { address, .. } => Some(address.clone()),
                _ => None,
            })
            .collect()
    }
}

#[derive(Clone, Copy)]
enum DetailPane {
    Registers,
    Memory,
}

impl DetailPane {
    fn toggle(self) -> Self {
        match self {
            Self::Registers => Self::Memory,
            Self::Memory => Self::Registers,
        }
    }
}

fn draw(frame: &mut ratatui::Frame<'_>, app: &App) {
    let outer = Layout::default()
        .direction(Direction::Vertical)
        .constraints([Constraint::Min(8), Constraint::Length(3)])
        .split(frame.area());
    let columns = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([Constraint::Percentage(56), Constraint::Percentage(44)])
        .split(outer[0]);
    draw_source(frame, columns[0], app);
    draw_machine(frame, columns[1], app);
    let help =
        Paragraph::new(" s/Enter step  n next  b/← back  c run  r reset  m regs/mem  q quit ")
            .style(Style::default().fg(Color::Black).bg(Color::Cyan))
            .block(Block::default().borders(Borders::ALL).title(" Keys "));
    frame.render_widget(help, outer[1]);
}

fn draw_source(frame: &mut ratatui::Frame<'_>, area: Rect, app: &App) {
    let view = app.view();
    let current = view.next_instruction.as_ref();
    let module = current
        .and_then(|location| {
            app.program
                .modules
                .iter()
                .find(|module| module.name == location.module)
        })
        .or_else(|| app.program.modules.first());
    let mut lines = Vec::new();
    if let Some(module) = module {
        for (index, source) in module.source.lines().enumerate() {
            let line_number = index + 1;
            let is_current = current.is_some_and(|location| {
                location.module == module.name && location.line == line_number
            });
            let style = if is_current {
                Style::default()
                    .fg(Color::Black)
                    .bg(Color::Yellow)
                    .add_modifier(Modifier::BOLD)
            } else {
                Style::default()
            };
            lines.push(Line::from(vec![
                Span::styled(if is_current { "▶" } else { " " }, style),
                Span::styled(format!(" {line_number:>3}  {source}"), style),
            ]));
        }
    }
    let title = module.map_or(" Source ".to_string(), |module| {
        format!(" Source: {} ", module.name)
    });
    let paragraph = Paragraph::new(Text::from(lines))
        .block(Block::default().borders(Borders::ALL).title(title))
        .wrap(Wrap { trim: false });
    frame.render_widget(paragraph, area);
}

fn draw_machine(frame: &mut ratatui::Frame<'_>, area: Rect, app: &App) {
    let rows = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(11),
            Constraint::Length(5),
            Constraint::Min(5),
        ])
        .split(area);
    let view = app.view();
    match app.detail {
        DetailPane::Registers => draw_registers(frame, rows[0], app, &view),
        DetailPane::Memory => draw_memory(frame, rows[0], app, &view),
    }

    let status = match &view.status {
        MachineStatus::Paused => "paused".to_string(),
        MachineStatus::Exited { shell_status, .. } => {
            format!("exited; shell status {shell_status}")
        }
        MachineStatus::Faulted { code, .. } => format!("faulted: {code}"),
    };
    let flags = format!(
        "CF={} PF={} AF={} ZF={} SF={} OF={}\nstatus: {}\nhistory: {} step(s)",
        u8::from(view.flags.cf),
        u8::from(view.flags.pf),
        u8::from(view.flags.af),
        u8::from(view.flags.zf),
        u8::from(view.flags.sf),
        u8::from(view.flags.of),
        status,
        view.history_depth
    );
    frame.render_widget(
        Paragraph::new(flags).block(
            Block::default()
                .borders(Borders::ALL)
                .title(" Flags / status "),
        ),
        rows[1],
    );

    let mut explanation = app.explanation().to_string();
    if let Some(result) = &app.last
        && !result.diagnostics.is_empty()
    {
        for diagnostic in &result.diagnostics {
            explanation.push_str(&format!("\n{}: {}", diagnostic.code, diagnostic.message));
        }
    }
    frame.render_widget(
        Paragraph::new(explanation)
            .block(Block::default().borders(Borders::ALL).title(" This step "))
            .wrap(Wrap { trim: true }),
        rows[2],
    );
}

fn draw_registers(frame: &mut ratatui::Frame<'_>, area: Rect, app: &App, view: &MachineView) {
    let changed = app.changed_registers();
    // One register per row remains legible in the 80-column terminals students
    // commonly use over SSH. `m` swaps this pane with memory and process output.
    let register_lines = view
        .registers
        .iter()
        .take(8)
        .map(|register| {
            let style = if changed.contains(&register.name) {
                Style::default()
                    .fg(Color::Yellow)
                    .add_modifier(Modifier::BOLD)
            } else {
                Style::default()
            };
            Line::from(Span::styled(
                format!(" {:>3}  {}", register.name, register.hex),
                style,
            ))
        })
        .collect::<Vec<_>>();
    frame.render_widget(
        Paragraph::new(register_lines).block(
            Block::default()
                .borders(Borders::ALL)
                .title(" Registers (m) "),
        ),
        area,
    );
}

fn draw_memory(frame: &mut ratatui::Frame<'_>, area: Rect, app: &App, view: &MachineView) {
    let changed = app.changed_memory();
    let mut lines = Vec::new();
    for symbol in &view.memory.symbols {
        let count = symbol.size.div_ceil(symbol.element_width);
        lines.push(Line::from(vec![
            Span::styled(
                format!(" {}", symbol.name),
                Style::default().add_modifier(Modifier::BOLD),
            ),
            Span::raw(format!(
                " {} × {count}",
                directive_name(symbol.element_width)
            )),
        ]));
        let bytes = &view.memory.bytes[symbol.offset..symbol.offset + symbol.size];
        let address = parse_hex(&symbol.address);
        if symbol.element_width == 1 && symbol.size > 8 {
            for (index, row) in bytes.chunks(8).enumerate() {
                lines.push(Line::from(format!(
                    " +{:02x}  {}",
                    index * 8,
                    format_bytes(row)
                )));
            }
        } else {
            for (index, element) in bytes.chunks(symbol.element_width).enumerate() {
                let element_address = address + (index * symbol.element_width) as u64;
                let address_text = format!("0x{element_address:016x}");
                let style = if changed.contains(&address_text) {
                    Style::default()
                        .fg(Color::Yellow)
                        .add_modifier(Modifier::BOLD)
                } else {
                    Style::default()
                };
                lines.push(Line::styled(
                    format!(" [{index}] signed {}", little_signed(element)),
                    style,
                ));
                lines.push(Line::styled(
                    format!("      {}", format_bytes(element)),
                    style,
                ));
            }
        }
    }
    if !view.io.stdout_bytes.is_empty() {
        lines.push(Line::styled(
            format!(" stdout: `{}`", view.io.stdout_escaped),
            Style::default().fg(Color::Cyan),
        ));
    }
    if !view.io.stderr_bytes.is_empty() {
        lines.push(Line::styled(
            format!(" stderr: `{}`", view.io.stderr_escaped),
            Style::default().fg(Color::Red),
        ));
    }
    if lines.is_empty() {
        lines.push(Line::from(" No .data or process output yet."));
    }
    frame.render_widget(
        Paragraph::new(lines).block(
            Block::default()
                .borders(Borders::ALL)
                .title(" Memory / output (m) "),
        ),
        area,
    );
}

fn directive_name(width: usize) -> &'static str {
    match width {
        1 => ".byte",
        2 => ".word",
        4 => ".long",
        8 => ".quad",
        _ => "data",
    }
}

fn format_bytes(bytes: &[u8]) -> String {
    bytes
        .iter()
        .map(|byte| format!("{byte:02x}"))
        .collect::<Vec<_>>()
        .join(" ")
}

fn little_signed(bytes: &[u8]) -> i128 {
    let value = bytes
        .iter()
        .enumerate()
        .fold(0_u128, |value, (index, byte)| {
            value | (u128::from(*byte) << (index * 8))
        });
    let bits = bytes.len() * 8;
    if bits > 0 && value & (1_u128 << (bits - 1)) != 0 {
        value as i128 - (1_i128 << bits)
    } else {
        value as i128
    }
}

fn parse_hex(value: &str) -> u64 {
    u64::from_str_radix(value.trim_start_matches("0x"), 16).unwrap_or(0)
}

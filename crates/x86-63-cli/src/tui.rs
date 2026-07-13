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
                KeyCode::Char('s') | KeyCode::Right | KeyCode::Enter => Some(Command::Step),
                KeyCode::Char('n') => Some(Command::Next),
                KeyCode::Char('b') | KeyCode::Left => Some(Command::Back),
                KeyCode::Char('r') => Some(Command::Reset),
                KeyCode::Char('c') => Some(Command::Continue { max_steps: 10_000 }),
                _ => None,
            };
            if let Some(command) = command {
                app.last = Some(app.session.execute(command));
            }
        }
    }
}

struct App {
    session: Session,
    program: ProgramView,
    last: Option<CommandResult>,
}

impl App {
    fn new(session: Session) -> Self {
        let program = session.program();
        Self {
            session,
            program,
            last: None,
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
    let help = Paragraph::new(" s/Enter step   n next   b/← back   c run   r reset   q quit ")
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
    let changed = app.changed_registers();
    // The lecture-3 programs use the original eight general-purpose registers.
    // One register per row remains legible in the 80-column terminals students
    // commonly use over SSH; later slices can add a toggle for r8-r15.
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
        Paragraph::new(register_lines)
            .block(Block::default().borders(Borders::ALL).title(" Registers ")),
        rows[0],
    );

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

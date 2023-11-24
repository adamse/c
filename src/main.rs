#![feature(let_chains)]

use std::error::Error;
use std::ops;
use crossterm::event::{Event, KeyCode, KeyModifiers};
use ratatui::{prelude::*, widgets::*};
use tui_input::backend::crossterm::EventHandler;

struct S {
    input: tui_input::Input,
    output: Res,
}

fn operator(op: &str) -> Option<fn(i64,i64) -> i64> {
    match op {
        "p" | "+" => Some(ops::Add::add),
        "m" | "*" => Some(ops::Mul::mul),
        "d" => Some(ops::Div::div),
        _ => None,
    }
}

#[derive(PartialEq, Eq, Clone, Copy, Default)]
enum Format {
    #[default]
    Dec,
    Hex
}

#[derive(Default)]
struct Res {
    stack: Vec<i64>,
    err: Option<String>,
    format: Format,
}

impl Res {
    fn render(&self) -> String {
        let mut out = "".to_owned();
        if self.format == Format::Dec {
            for &x in self.stack.iter() {
                out.push_str(&format!("{} ", x));
            }
        } else {
            for &x in self.stack.iter() {
                out.push_str(&format!("{:#x} ", x));
            }
        }

        out
    }
}


fn parse(inp: &str) -> Res {
    let mut stack = vec![];
    let mut err = None;
    let mut format = Format::Dec;

    for x in inp.split_whitespace() {
        if !x.is_ascii() {
            // handle later
            continue;
        }

        if let Some(x) = x.strip_prefix("0x") &&
            let Ok(num) = i64::from_str_radix(x, 16) {
            stack.push(num);
            format = Format::Hex;
            continue;
        }

        if let Ok(num) = x.parse() {
            stack.push(num);
            continue;
        }

        if x.len() < 1 {
            continue;
        }

        let (head, rest) = x.split_at(1);

        match head {
            // iota, ( n --- 1 .. n )
            "i" => {
                if let Some(count) = stack.pop() {
                    stack.extend(1..=count);
                } else {
                    err = Some("i needs a number".into());
                }
                continue;
            },
            // fold, /op, ( a b .. x --- a op b op .. op x )
            "/" => {
                if let Some(op) = operator(rest) {
                    let res = stack.iter().copied().reduce(op).unwrap();
                    stack.truncate(0);
                    stack.push(res);
                } else {
                    err = Some("/<op>".into())
                }
                continue;

            }
            "." => {
                match rest {
                    "h" => { format = Format::Hex; },
                    "d" => { format = Format::Dec; },
                    _ => {},
                }
                continue;
            },
            _ => {},
        }

        if let Some(op) = operator(head) &&
            let Some(a) = stack.pop() &&
            let Some(b) = stack.pop() {

            stack.push(op(a,b));

            continue;
        }

        err = Some(format!("couldn't parse '{x}'"));

    }

    Res {
        stack,
        err,
        format,
    }
}

fn main() -> Result<(), Box<dyn Error>> {
    crossterm::terminal::enable_raw_mode()?;

    let stdout = std::io::stdout();
    let backend = CrosstermBackend::new(stdout);
    let mut terminal = Terminal::with_options(
        backend,
        TerminalOptions {
            viewport: Viewport::Inline(3),
        })?;

    let state = S {
        input: Default::default(),
        output: Default::default(),
    };

    run_app(&mut terminal, state)?;

    crossterm::terminal::disable_raw_mode()?;

    Ok(())
}

fn run_app<B: Backend>(term: &mut Terminal<B>, mut s: S) -> Result<(), Box<dyn Error>> {
    loop {
        term.draw(|frame| ui(frame, &s))?;

        match crossterm::event::read()? {
            ref ev@Event::Key(key) => {
                if key.code == KeyCode::Char('d') && key.modifiers == KeyModifiers::CONTROL {
                    // exit on C-d
                    return Ok(())
                } else if key.code == KeyCode::Enter {
                    term.insert_before(1, |buf| {
                        Paragraph::new(s.output.render()).render(buf.area, buf);
                    })?;
                    s.input.reset();
                } else {
                    s.input.handle_event(ev);
                }
            },
            // Event::FocusGained => todo!(),
            // Event::FocusLost => todo!(),
            // Event::Mouse(_) => todo!(),
            // Event::Paste(_) => todo!(),
            // Event::Resize(_, _) => todo!(),
            _ => {},
        }

        s.output = parse(s.input.value());

    }
}

fn ui(f: &mut Frame, s: &S) {

    let chunks = Layout::default()
        .constraints([Constraint::Max(1), Constraint::Max(1), Constraint::Max(1)])
        .split(f.size());

    // error message
    if let Some(ref err) = s.output.err {
        let error = Paragraph::new(err.clone());
        f.render_widget(error, chunks[0]);
    }

    // current output
    let output = Paragraph::new(s.output.render());
    f.render_widget(output, chunks[1]);

    let input_chunks = Layout::default()
        .constraints([Constraint::Length(2), Constraint::Min(1)])
        .direction(Direction::Horizontal)
        .split(chunks[2]);

    // > prompt
    let prompt = Paragraph::new("> ");
    f.render_widget(prompt, input_chunks[0]);

    // input
    let scroll = s.input.visual_scroll(input_chunks[1].width as usize - 1);
    let input = Paragraph::new(s.input.value())
        .scroll((0, scroll as u16));
    f.render_widget(input, input_chunks[1]);
    f.set_cursor(
        input_chunks[1].x +
            ((s.input.visual_cursor()).max(scroll) - scroll) as u16,
        input_chunks[1].y);

}

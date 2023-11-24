#![feature(let_chains)]

use std::error::Error;
use std::ops;
use crossterm::event::{Event, KeyCode, KeyModifiers};
use ratatui::{prelude::*, widgets::*};
use tui_input::backend::crossterm::EventHandler;

struct S {
    input: tui_input::Input,
    output: Result<Vec<i64>, String>,
}

fn operator(op: &str) -> Option<fn(i64,i64) -> i64> {
    match op {
        "p" | "+" => Some(ops::Add::add),
        "m" | "*" => Some(ops::Mul::mul),
        _ => None,
    }
}

fn parse(inp: &str) -> Result<Vec<i64>, String> {
    let mut stack = vec![];

    for x in inp.split_whitespace() {
        if !x.is_ascii() {
            // handle later
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
                let count = stack.pop().ok_or("")?;
                stack.extend(1..=count);
                continue;
            },
            // fold, /op, ( a b .. x --- a op b op .. op x )
            "/" => {
                if let Some(op) = operator(rest) {
                    let res = stack.iter().copied().reduce(op).unwrap();
                    stack.truncate(0);
                    stack.push(res);
                }
                continue;

            }
            _ => {},
        }

        if let Some(op) = operator(head) &&
            let Some(a) = stack.pop() &&
            let Some(b) = stack.pop() {

            stack.push(op(a,b));
        }

    }

    Ok(stack)
}

fn render(stack: &[i64]) -> String {
    let mut out = "".to_owned();
    for x in stack {
        out.push_str(&format!("{} ", x));
    }

    out
}

fn main() -> Result<(), Box<dyn Error>> {
    crossterm::terminal::enable_raw_mode()?;

    let stdout = std::io::stdout();
    let backend = CrosstermBackend::new(stdout);
    let mut terminal = Terminal::with_options(
        backend,
        TerminalOptions {
            viewport: Viewport::Inline(2),
        })?;

    let state = S {
        input: Default::default(),
        output: Ok(vec![]),
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
                        Paragraph::new(match s.output {
                            Ok(stack) => {
                                render(&stack[..])
                            },
                            Err(err) => {
                                err
                            },
                        }).render(buf.area, buf);
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
        .constraints([Constraint::Percentage(50), Constraint::Percentage(50)])
        .split(f.size());

    // current output
    let output = Paragraph::new(match &s.output {
        Ok(stack) => {
            render(&stack[..])
        },
        Err(err) => {
            err.clone()
        },
    });
    f.render_widget(output, chunks[0]);

    let input_chunks = Layout::default()
        .constraints([Constraint::Length(2), Constraint::Min(1)])
        .direction(Direction::Horizontal)
        .split(chunks[1]);

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

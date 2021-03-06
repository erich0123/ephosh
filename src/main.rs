use std::io;
use termion::raw::IntoRawMode;
use tui::Terminal;
use tui::backend::TermionBackend;
use tui::widgets::{Block, Borders, Paragraph, List, ListItem, Table, Row};
use tui::text::{Span, Spans};
use tui::layout::{Layout, Constraint, Direction};
use termion::event::Key;
use std::{
    env,
    sync::mpsc::{self, Sender, Receiver, TryRecvError},
    path::Path
};

mod event;
mod non_blocking;

fn main() -> Result<(), io::Error> {
    let mut output_pane: Vec<String> = vec![];

    let mut output_receivers: Vec<Receiver<String>> = vec![];

    std::process::Command::new("clear").spawn().unwrap();

    let mut it = String::from("");

    let stdout = io::stdout().into_raw_mode()?;
    let backend = TermionBackend::new(stdout);
    let mut terminal = Terminal::new(backend)?;
    let events = event::Events::new();

    loop {
        terminal.draw(|f| {
            let chunks = Layout::default()
                .direction(Direction::Vertical)
                .margin(1)
                .constraints([
                    Constraint::Percentage(94),
                    Constraint::Percentage(6),
                ].as_ref())
                .split(f.size());

            let block = Block::default()
                .title("Input")
                .borders(Borders::ALL);
            f.render_widget(block, chunks[1]);

            f.set_cursor(chunks[1].x + 1 + it.len() as u16, chunks[1].y+1);


            let mut output_newlines: Vec<Vec<&str>> = vec![];

            for output in &output_pane {
                let newlines: Vec<&str> = output.split("\n").collect();
                output_newlines.push(newlines);
            }

            let mut lists_vec: Vec<List> = vec![];
            
            for newlines in output_newlines {
                let list_items: Vec<ListItem> = newlines.iter().map(|element| ListItem::new(Spans::from(Span::raw(element.to_owned())))).collect();
                
                let list = List::new(list_items).block(Block::default().borders(Borders::ALL));
                
                lists_vec.push(list);
            }

            let output_pane_len = match lists_vec.len() {
                0 => 1,
                _ => lists_vec.len()
            };

            let percentage = (100 / output_pane_len) as u16;

            let mut constraints: Vec<Constraint> = vec![];

            for _ in &lists_vec {
                let constraint = Constraint::Percentage(percentage);
                constraints.push(constraint);
            }

            let output_layout = Layout::default()
                .direction(Direction::Horizontal)
                .margin(1)
                .constraints(constraints)
                .split(chunks[0]);

            for (i, list) in lists_vec.iter().enumerate() {
                f.render_widget(list.clone(), output_layout[i]);
            }

            drop(lists_vec);

            let inp = Paragraph::new(it.as_ref())
                .block(Block::default()
                    .borders(Borders::ALL).title("Input"));
                    f.render_widget(inp, chunks[1]);
        }).unwrap();

        if let event::Event::Input(input) = events.next().unwrap() {
            match input {
                Key::Char('\n') => {
                    let args: Vec<&str> = it.split_whitespace().collect();
                    let mut cmd = "".to_owned();

                    if args.is_empty(){
                        continue;
                    }

                    if args[0] == "clear" {
                        output_pane.clear();
                        it.clear();
                        continue;
                    }

                    if args[0] == "exit" {
                        break;
                    }

                    let path_var_result = env::var("PATH");

                    if let Err(err) = path_var_result {
                        output_pane.push(err.to_string());
                        continue;
                    }

                    let path_var = path_var_result.unwrap();

                    let mut paths_vec: Vec<String> = path_var.split(":")
                        .collect::<Vec<&str>>()
                        .into_iter()
                        .map(|s| s.to_owned()).collect();

                    for path in paths_vec.iter_mut() {
                        path.push_str("/");
                        path.push_str(args[0]);

                        if Path::new(&path).exists() {
                            cmd = path.to_string();
                        }
                    }

                    let command_result = non_blocking::Command::new(&cmd).args(&args[1..]).spawn();

                    if let Err(err) = command_result {
                        output_pane.push(err.to_string());
                        continue;
                    }

                    let (_sender, receiver) = command_result.unwrap();

                    output_receivers.push(receiver);

                    it.clear();
                }
                Key::Char('q') => {
                    break;
                }
                Key::Char(c) => {
                    it.push(c);
                }
                Key::Backspace => {
                    it.pop();
                },
                _ => {}
            }
        }

        let mut i: usize = 0;
        while i < output_receivers.len(){
            let receiver = &output_receivers[i];

            match receiver.try_recv() {
                Ok(message) => {
                    if message != ""{
                        output_pane.push(message)
                    }
                },

                Err(err) => {
                    if err == TryRecvError::Disconnected {
                        output_receivers.remove(i);
                    }
                }
            };

            i += 1;
        }
    }
    Ok(())
}

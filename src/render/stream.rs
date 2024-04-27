use super::{MarkdownRender, ReplyEvent};

use crate::utils::{run_spinner, AbortSignal};

use anyhow::Result;
use crossterm::{
    cursor,
    event::{self, Event, KeyCode, KeyModifiers},
    queue, style,
    terminal::{self, disable_raw_mode, enable_raw_mode},
};
use std::{
    io::{self, stdout, Stdout, Write},
    time::Duration,
};
use textwrap::core::display_width;
use tokio::sync::{mpsc::UnboundedReceiver, oneshot};

pub async fn markdown_stream(
    rx: UnboundedReceiver<ReplyEvent>,
    render: &mut MarkdownRender,
    abort: &AbortSignal,
) -> Result<()> {
    enable_raw_mode()?;
    let mut stdout = io::stdout();

    let ret = markdown_stream_inner(rx, render, abort, &mut stdout).await;

    disable_raw_mode()?;

    ret
}

pub async fn raw_stream(mut rx: UnboundedReceiver<ReplyEvent>, abort: &AbortSignal) -> Result<()> {
    loop {
        if abort.aborted() {
            return Ok(());
        }
        if let Some(evt) = rx.recv().await {
            match evt {
                ReplyEvent::Text(text) => {
                    print!("{}", text);
                    stdout().flush()?;
                }
                ReplyEvent::Done => {
                    break;
                }
            }
        }
    }
    Ok(())
}

async fn markdown_stream_inner(
    mut rx: UnboundedReceiver<ReplyEvent>,
    render: &mut MarkdownRender,
    abort: &AbortSignal,
    writer: &mut Stdout,
) -> Result<()> {
    let mut buffer = String::new();
    let mut buffer_rows = 1;

    let columns = terminal::size()?.0;

    let (spinner_tx, spinner_rx) = oneshot::channel();
    let mut spinner_tx = Some(spinner_tx);
    tokio::spawn(run_spinner(" Generating", spinner_rx));

    'outer: loop {
        if abort.aborted() {
            return Ok(());
        }
        for reply_event in gather_events(&mut rx).await {
            if let Some(spinner_tx) = spinner_tx.take() {
                let _ = spinner_tx.send(());
            }

            match reply_event {
                ReplyEvent::Text(mut text) => {
                    // tab width hacking
                    text = text.replace('\t', "    ");

                    let (col, mut row) = cursor::position()?;

                    // Fix unexpected duplicate lines on kitty, see https://github.com/sigoden/aichat/issues/105
                    if col == 0 && row > 0 && display_width(&buffer) == columns as usize {
                        row -= 1;
                    }

                    if row + 1 >= buffer_rows {
                        queue!(writer, cursor::MoveTo(0, row + 1 - buffer_rows),)?;
                    } else {
                        let scroll_rows = buffer_rows - row - 1;
                        queue!(
                            writer,
                            terminal::ScrollUp(scroll_rows),
                            cursor::MoveTo(0, 0),
                        )?;
                    }

                    // No guarantee that text returned by render will not be re-layouted, so it is better to clear it.
                    queue!(writer, terminal::Clear(terminal::ClearType::FromCursorDown))?;

                    if text.contains('\n') {
                        let text = format!("{buffer}{text}");
                        let (head, tail) = split_line_tail(&text);
                        let output = render.render(head);
                        print_block(writer, &output, columns)?;
                        buffer = tail.to_string();
                    } else {
                        buffer = format!("{buffer}{text}");
                    }

                    let output = render.render_line(&buffer);
                    if output.contains('\n') {
                        let (head, tail) = split_line_tail(&output);
                        buffer_rows = print_block(writer, head, columns)?;
                        queue!(writer, style::Print(&tail),)?;

                        // No guarantee the buffer width of the buffer will not exceed the number of columns.
                        // So we calculate the number of rows needed, rather than setting it directly to 1.
                        buffer_rows += need_rows(tail, columns);
                    } else {
                        queue!(writer, style::Print(&output))?;
                        buffer_rows = need_rows(&output, columns);
                    }

                    writer.flush()?;
                }
                ReplyEvent::Done => {
                    break 'outer;
                }
            }
        }

        if crossterm::event::poll(Duration::from_millis(25))? {
            if let Event::Key(key) = event::read()? {
                match key.code {
                    KeyCode::Char('c') if key.modifiers == KeyModifiers::CONTROL => {
                        abort.set_ctrlc();
                        break;
                    }
                    KeyCode::Char('d') if key.modifiers == KeyModifiers::CONTROL => {
                        abort.set_ctrld();
                        break;
                    }
                    _ => {}
                }
            }
        }
    }

    if let Some(spinner_tx) = spinner_tx.take() {
        let _ = spinner_tx.send(());
    }
    Ok(())
}

async fn gather_events(rx: &mut UnboundedReceiver<ReplyEvent>) -> Vec<ReplyEvent> {
    let mut texts = vec![];
    let mut done = false;
    tokio::select! {
        _ = async {
            while let Some(reply_event) = rx.recv().await {
                match reply_event {
                    ReplyEvent::Text(v) => texts.push(v),
                    ReplyEvent::Done => {
                        done = true;
                        break;
                    }
                }
            }
        } => {}
        _ = tokio::time::sleep(Duration::from_millis(50)) => {}
    };
    let mut events = vec![];
    if !texts.is_empty() {
        events.push(ReplyEvent::Text(texts.join("")))
    }
    if done {
        events.push(ReplyEvent::Done)
    }
    events
}

fn print_block(writer: &mut Stdout, text: &str, columns: u16) -> Result<u16> {
    let mut num = 0;
    for line in text.split('\n') {
        queue!(
            writer,
            style::Print(line),
            style::Print("\n"),
            cursor::MoveLeft(columns),
        )?;
        num += 1;
    }
    Ok(num)
}

fn split_line_tail(text: &str) -> (&str, &str) {
    if let Some((head, tail)) = text.rsplit_once('\n') {
        (head, tail)
    } else {
        ("", text)
    }
}

fn need_rows(text: &str, columns: u16) -> u16 {
    let buffer_width = display_width(text).max(1) as u16;
    (buffer_width + columns - 1) / columns
}

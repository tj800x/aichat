mod completer;
mod highlighter;
mod prompt;

use self::completer::ReplCompleter;
use self::highlighter::ReplHighlighter;
use self::prompt::ReplPrompt;

use crate::client::{ensure_model_capabilities, init_client, send_stream};
use crate::config::{GlobalConfig, Input, InputContext, State};
use crate::render::render_error;
use crate::utils::{create_abort_signal, set_text, AbortSignal};

use anyhow::{bail, Context, Result};
use fancy_regex::Regex;
use lazy_static::lazy_static;
use nu_ansi_term::Color;
use reedline::{
    default_emacs_keybindings, default_vi_insert_keybindings, default_vi_normal_keybindings,
    ColumnarMenu, EditCommand, EditMode, Emacs, KeyCode, KeyModifiers, Keybindings, Reedline,
    ReedlineEvent, ReedlineMenu, ValidationResult, Validator, Vi,
};
use reedline::{MenuBuilder, Signal};
use std::{env, process};

const MENU_NAME: &str = "completion_menu";

lazy_static! {
    static ref REPL_COMMANDS: [ReplCommand; 16] = [
        ReplCommand::new(".help", "Show this help message", State::all()),
        ReplCommand::new(".info", "View system info", State::all()),
        ReplCommand::new(".model", "Change the current LLM", State::all()),
        ReplCommand::new(
            ".prompt",
            "Make a temporary role using a prompt",
            State::able_change_role()
        ),
        ReplCommand::new(
            ".role",
            "Switch to a specific role",
            State::able_change_role()
        ),
        ReplCommand::new(".info role", "View role info", State::in_role(),),
        ReplCommand::new(".exit role", "Leave the role", State::in_role(),),
        ReplCommand::new(".session", "Begin a chat session", State::not_in_session(),),
        ReplCommand::new(".info session", "View session info", State::in_session(),),
        ReplCommand::new(
            ".save session",
            "Save the chat to file",
            State::in_session(),
        ),
        ReplCommand::new(
            ".clear messages",
            "Erase messages in the current session",
            State::unable_change_role()
        ),
        ReplCommand::new(
            ".exit session",
            "End the current session",
            State::in_session(),
        ),
        ReplCommand::new(".file", "Include files with the message", State::all()),
        ReplCommand::new(".set", "Adjust settings", State::all()),
        ReplCommand::new(".copy", "Copy the last response", State::all()),
        ReplCommand::new(".exit", "Exit the REPL", State::all()),
    ];
    static ref COMMAND_RE: Regex = Regex::new(r"^\s*(\.\S*)\s*").unwrap();
    static ref MULTILINE_RE: Regex = Regex::new(r"(?s)^\s*:::\s*(.*)\s*:::\s*$").unwrap();
}

pub struct Repl {
    config: GlobalConfig,
    editor: Reedline,
    prompt: ReplPrompt,
    abort: AbortSignal,
}

impl Repl {
    pub fn init(config: &GlobalConfig) -> Result<Self> {
        let editor = Self::create_editor(config)?;

        let prompt = ReplPrompt::new(config);

        let abort = create_abort_signal();

        Ok(Self {
            config: config.clone(),
            editor,
            prompt,
            abort,
        })
    }

    pub async fn run(&mut self) -> Result<()> {
        self.banner();

        loop {
            if self.abort.aborted_ctrld() {
                break;
            }
            let sig = self.editor.read_line(&self.prompt);
            match sig {
                Ok(Signal::Success(line)) => {
                    self.abort.reset();
                    match self.handle(&line).await {
                        Ok(exit) => {
                            if exit {
                                break;
                            }
                        }
                        Err(err) => {
                            render_error(err, self.config.read().highlight);
                            println!()
                        }
                    }
                }
                Ok(Signal::CtrlC) => {
                    self.abort.set_ctrlc();
                    println!("(To exit, press Ctrl+D or type .exit)\n");
                }
                Ok(Signal::CtrlD) => {
                    self.abort.set_ctrld();
                    break;
                }
                _ => {}
            }
        }
        self.handle(".exit session").await?;
        Ok(())
    }

    async fn handle(&self, mut line: &str) -> Result<bool> {
        if let Ok(Some(captures)) = MULTILINE_RE.captures(line) {
            if let Some(text_match) = captures.get(1) {
                line = text_match.as_str();
            }
        }
        match parse_command(line) {
            Some((cmd, args)) => match cmd {
                ".help" => {
                    dump_repl_help();
                }
                ".info" => match args {
                    Some("role") => {
                        let info = self.config.read().role_info()?;
                        println!("{}", info);
                    }
                    Some("session") => {
                        let info = self.config.read().session_info()?;
                        println!("{}", info);
                    }
                    Some(_) => unknown_command()?,
                    None => {
                        let output = self.config.read().system_info()?;
                        println!("{}", output);
                    }
                },
                ".model" => match args {
                    Some(name) => {
                        self.config.write().set_model(name)?;
                    }
                    None => println!("Usage: .model <name>"),
                },
                ".prompt" => match args {
                    Some(text) => {
                        self.config.write().set_prompt(text)?;
                    }
                    None => println!("Usage: .prompt <text>..."),
                },
                ".role" => match args {
                    Some(args) => match args.split_once(|c| c == '\n' || c == ' ') {
                        Some((name, text)) => {
                            let role = self.config.read().retrieve_role(name.trim())?;
                            let input =
                                Input::from_str(text.trim(), InputContext::new(Some(role), false));
                            self.ask(input).await?;
                        }
                        None => {
                            self.config.write().set_role(args)?;
                        }
                    },
                    None => println!(r#"Usage: .role <name> [text]..."#),
                },
                ".session" => {
                    self.config.write().start_session(args)?;
                }
                ".save" => {
                    match args.map(|v| match v.split_once(' ') {
                        Some((subcmd, args)) => (subcmd, args.trim()),
                        None => (v, ""),
                    }) {
                        Some(("session", name)) => {
                            self.config.write().save_session(name)?;
                        }
                        _ => {
                            println!(r#"Usage: .save session [name]"#)
                        }
                    }
                }
                ".set" => match args {
                    Some(args) => {
                        self.config.write().update(args)?;
                    }
                    _ => {
                        println!("Usage: .set <key> <value>...")
                    }
                },
                ".copy" => {
                    let config = self.config.read();
                    self.copy(config.last_reply())
                        .with_context(|| "Failed to copy the last output")?;
                }
                ".file" => match args {
                    Some(args) => {
                        let (files, text) = match args.split_once(" -- ") {
                            Some((files, text)) => (files.trim(), text.trim()),
                            None => (args, ""),
                        };
                        let files = shell_words::split(files).with_context(|| "Invalid args")?;
                        let input = Input::new(text, files, self.config.read().input_context())?;
                        self.ask(input).await?;
                    }
                    None => println!("Usage: .file <files>... [-- <text>...]"),
                },
                ".exit" => match args {
                    Some("role") => {
                        self.config.write().clear_role()?;
                    }
                    Some("session") => {
                        self.config.write().end_session()?;
                    }
                    Some(_) => unknown_command()?,
                    None => {
                        return Ok(true);
                    }
                },
                ".clear" => match args {
                    Some("messages") => {
                        self.config.write().clear_session_messages()?;
                    }
                    _ => unknown_command()?,
                },
                _ => unknown_command()?,
            },
            None => {
                let input = Input::from_str(line, self.config.read().input_context());
                self.ask(input).await?;
            }
        }

        println!();

        Ok(false)
    }

    async fn ask(&self, input: Input) -> Result<()> {
        if input.is_empty() {
            return Ok(());
        }
        while self.config.read().is_compressing_session() {
            std::thread::sleep(std::time::Duration::from_millis(100));
        }
        self.config.read().maybe_print_send_tokens(&input);
        let mut client = init_client(&self.config)?;
        ensure_model_capabilities(client.as_mut(), input.required_capabilities())?;
        let output = send_stream(&input, client.as_ref(), &self.config, self.abort.clone()).await?;
        self.config.write().save_message(input, &output)?;
        self.config.read().maybe_copy(&output);
        if self.config.write().should_compress_session() {
            let config = self.config.clone();
            let color = if config.read().light_theme {
                Color::LightGray
            } else {
                Color::DarkGray
            };
            print!(
                "\n📢 {}{}{}\n",
                color.normal().paint(
                    "Session compression is being activated because the current tokens exceed `"
                ),
                color.italic().paint("compress_threshold"),
                color.normal().paint("`."),
            );
            tokio::spawn(async move {
                let _ = compress_session(&config).await;
                config.write().end_compressing_session();
            });
        }
        Ok(())
    }

    fn banner(&self) {
        let version = env!("CARGO_PKG_VERSION");
        print!(
            r#"Welcome to aichat {version}
Type ".help" for more information.
"#
        )
    }

    fn create_editor(config: &GlobalConfig) -> Result<Reedline> {
        let completer = ReplCompleter::new(config);
        let highlighter = ReplHighlighter::new(config);
        let menu = Self::create_menu();
        let edit_mode = Self::create_edit_mode(config);
        let mut editor = Reedline::create()
            .with_completer(Box::new(completer))
            .with_highlighter(Box::new(highlighter))
            .with_menu(menu)
            .with_edit_mode(edit_mode)
            .with_quick_completions(true)
            .with_partial_completions(true)
            .use_bracketed_paste(true)
            .with_validator(Box::new(ReplValidator))
            .with_ansi_colors(true);

        if let Some(cmd) = config.read().buffer_editor() {
            let temp_file =
                env::temp_dir().join(format!("aichat-{}.txt", chrono::Utc::now().timestamp()));
            let command = process::Command::new(cmd);
            editor = editor.with_buffer_editor(command, temp_file);
        }

        Ok(editor)
    }

    fn extra_keybindings(keybindings: &mut Keybindings) {
        keybindings.add_binding(
            KeyModifiers::NONE,
            KeyCode::Tab,
            ReedlineEvent::UntilFound(vec![
                ReedlineEvent::Menu(MENU_NAME.to_string()),
                ReedlineEvent::MenuNext,
            ]),
        );
        keybindings.add_binding(
            KeyModifiers::SHIFT,
            KeyCode::BackTab,
            ReedlineEvent::MenuPrevious,
        );
        keybindings.add_binding(
            KeyModifiers::CONTROL,
            KeyCode::Enter,
            ReedlineEvent::Edit(vec![EditCommand::InsertNewline]),
        );
    }

    fn create_edit_mode(config: &GlobalConfig) -> Box<dyn EditMode> {
        let edit_mode: Box<dyn EditMode> = if config.read().keybindings.is_vi() {
            let mut normal_keybindings = default_vi_normal_keybindings();
            let mut insert_keybindings = default_vi_insert_keybindings();
            Self::extra_keybindings(&mut normal_keybindings);
            Self::extra_keybindings(&mut insert_keybindings);
            Box::new(Vi::new(insert_keybindings, normal_keybindings))
        } else {
            let mut keybindings = default_emacs_keybindings();
            Self::extra_keybindings(&mut keybindings);
            Box::new(Emacs::new(keybindings))
        };
        edit_mode
    }

    fn create_menu() -> ReedlineMenu {
        let completion_menu = ColumnarMenu::default().with_name(MENU_NAME);
        ReedlineMenu::EngineCompleter(Box::new(completion_menu))
    }

    fn copy(&self, text: &str) -> Result<()> {
        if text.is_empty() {
            bail!("Empty text")
        }
        set_text(text)?;
        Ok(())
    }
}

#[derive(Debug, Clone)]
pub struct ReplCommand {
    name: &'static str,
    description: &'static str,
    valid_states: Vec<State>,
}

impl ReplCommand {
    fn new(name: &'static str, desc: &'static str, valid_states: Vec<State>) -> Self {
        Self {
            name,
            description: desc,
            valid_states,
        }
    }

    fn is_valid(&self, state: &State) -> bool {
        self.valid_states.contains(state)
    }
}

/// A default validator which checks for mismatched quotes and brackets
struct ReplValidator;

impl Validator for ReplValidator {
    fn validate(&self, line: &str) -> ValidationResult {
        let line = line.trim();
        if line.starts_with(r#":::"#) && !line[3..].ends_with(r#":::"#) {
            ValidationResult::Incomplete
        } else {
            ValidationResult::Complete
        }
    }
}

fn unknown_command() -> Result<()> {
    bail!(r#"Unknown command. Type ".help" for more information."#);
}

fn dump_repl_help() {
    let head = REPL_COMMANDS
        .iter()
        .map(|cmd| format!("{:<24} {}", cmd.name, cmd.description))
        .collect::<Vec<String>>()
        .join("\n");
    println!(
        r###"{head}

Type ::: to start multi-line editing, type ::: to finish it.
Press Ctrl+O to open an editor to edit line input.
Press Ctrl+C to cancel the response, Ctrl+D to exit the REPL"###,
    );
}

fn parse_command(line: &str) -> Option<(&str, Option<&str>)> {
    match COMMAND_RE.captures(line) {
        Ok(Some(captures)) => {
            let cmd = captures.get(1)?.as_str();
            let args = line[captures[0].len()..].trim();
            let args = if args.is_empty() { None } else { Some(args) };
            Some((cmd, args))
        }
        _ => None,
    }
}

async fn compress_session(config: &GlobalConfig) -> Result<()> {
    let input = Input::from_str(
        config.read().summarize_prompt(),
        config.read().input_context(),
    );
    let mut client = init_client(config)?;
    ensure_model_capabilities(client.as_mut(), input.required_capabilities())?;
    let summary = client.send_message(input).await?;
    config.write().compress_session(&summary);
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_process_command_line() {
        assert_eq!(parse_command(" ."), Some((".", None)));
        assert_eq!(parse_command(" .role"), Some((".role", None)));
        assert_eq!(parse_command(" .role  "), Some((".role", None)));
        assert_eq!(
            parse_command(" .set dry_run true"),
            Some((".set", Some("dry_run true")))
        );
        assert_eq!(
            parse_command(" .set dry_run true  "),
            Some((".set", Some("dry_run true")))
        );
        assert_eq!(
            parse_command(".prompt \nabc\n"),
            Some((".prompt", Some("abc")))
        );
    }
}

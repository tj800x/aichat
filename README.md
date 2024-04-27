# Aichat: All-in-one AI-Powered CLI Chat & Copilot

[![CI](https://github.com/sigoden/aichat/actions/workflows/ci.yaml/badge.svg)](https://github.com/sigoden/aichat/actions/workflows/ci.yaml)
[![Crates](https://img.shields.io/crates/v/aichat.svg)](https://crates.io/crates/aichat)
[![Discord](https://img.shields.io/discord/1226737085453701222?label=Discord)](https://discord.gg/dSHTvH6S)

Aichat is a AI-powered CLI chat and copilot tool that seamlessly integrates with over 10 leading AI platforms, providing a powerful combination of chat-based interaction, context-aware conversations, and AI-assisted shell capabilities, all within a customizable and user-friendly environment.

![command mode](https://github.com/sigoden/aichat/assets/4012553/2ab27e1b-4078-4ea3-a98f-591b36491685)

![chat-repl mode](https://github.com/sigoden/aichat/assets/4012553/13427d54-efd5-4f4c-b17b-409edd30dfa3)

## Key Features

*  **Converse with Advanced AI:** Access and interact with 10+ leading AI platforms including OpenAI, Claude, Gemini, and more, all within one interface.
*  **Streamline Your Workflow:** Generate code, execute shell commands using natural language, and automate tasks with AI assistance.
*  **Unleash Your Creativity:** Utilize AI for writing, translation, image analysis, and exploring new ideas.
*  **Customize Your Experience:** Configure settings, create custom roles for AI, and personalize your chat interface. 
*  **Empower Your Terminal:** Integrate AI into your shell for intelligent autocompletion and command suggestions.
*  **Context & Session Management:** Maintain context within conversations and manage multiple sessions effortlessly.

## Supported AI Platforms

- OpenAI GPT-3.5/GPT-4 (paid, vision)
- Azure OpenAI (paid)
- OpenAI-Compatible platforms
- Gemini: Gemini-1.0/Gemini-1.5 (free, vision)
- VertexAI (paid, vision)
- Claude: Claude-3 (vision, paid)
- Mistral (paid)
- Cohere (paid)
- Ollama (free, local)
- Ernie (paid)
- Qianwen (paid, vision)

## Install

### Package Managers

- **Rust Developers:** `cargo install aichat`
- **Homebrew/Linuxbrew Users:** `brew install aichat`
- **Pacman Users**: `yay -S aichat`
- **Windows Scoop Users:** `scoop install aichat`
- **Android Termux Users:** `pkg install aichat`

### Pre-built Binaries

Download pre-built binaries for macOS, Linux, and Windows from [GitHub Releases](https://github.com/sigoden/aichat/releases), extract them, and add the `aichat` binary to your `$PATH`.

## Configuration

Upon first launch, Aichat will guide you through the configuration process. An example configuration file is provided below:

```
> No config file, create a new one? Yes
> AI Platform: openai
> API Key: <your_api_key_here>
✨ Saved config file to <config-dir>/aichat/config.yaml
```

Feel free to adjust the configuration according to your needs.

> Get `config.yaml` path with command `aichat --info` or repl command `.info`.

```yaml
model: openai:gpt-3.5-turbo      # Specify the language model to use
temperature: null                # Set default temperature parameter
top_p: null                      # Set default top-p parameter
save: true                       # Indicates whether to persist the message
save_session: null               # Controls the persistence of the session, if null, asking the user
highlight: true                  # Controls syntax highlighting
light_theme: false               # Activates a light color theme when true
wrap: no                         # Controls text wrapping (no, auto, <max-width>)
wrap_code: false                 # Enables or disables wrapping of code blocks
auto_copy: false                 # Enables or disables automatic copying the last LLM response to the clipboard 
keybindings: emacs               # Choose keybinding style (emacs, vi)
prelude: null                    # Set a default role or session to start with (role:<name>, session:<name>)

# Command that will be used to edit the current line buffer with ctrl+o
# if unset fallback to $EDITOR and $VISUAL
buffer_editor: null

# Compress session when token count reaches or exceeds this threshold (must be at least 1000)
compress_threshold: 1000

clients:
  - type: openai
    api_key: sk-xxx

  - type: openai-compatible
    name: localai
    api_base: http://127.0.0.1:8080/v1
    models:
      - name: llama3
        max_input_tokens: 8192
```

Refer to the [config.example.yaml](config.example.yaml) file for a complete list of configuration options. Environment variables can also be used for configuration; see the [Environment Variables](https://github.com/sigoden/aichat/wiki/Environment-Variables) page for details.

## Command line

```
Usage: aichat [OPTIONS] [TEXT]...

Arguments:
  [TEXT]...  Input text

Options:
  -m, --model <MODEL>        Select a LLM model
  -r, --role <ROLE>          Select a role
  -s, --session [<SESSION>]  Start or join a session
      --save-session         Forces the session to be saved
  -e, --execute              Execute commands in natural language
  -c, --code                 Output code only
  -f, --file <FILE>          Include files with the message
  -H, --no-highlight         Turn off syntax highlighting
  -S, --no-stream            Turns off stream mode
  -w, --wrap <WRAP>          Control text wrapping (no, auto, <max-width>)
      --light-theme          Use light theme
      --dry-run              Display the message without sending it
      --info                 Display information
      --list-models          List all available models
      --list-roles           List all available roles
      --list-sessions        List all available sessions
  -h, --help                 Print help
  -V, --version              Print version
```

Here are some practical examples:

```sh
aichat                                          # Start REPL

aichat -e install nvim                          # Execute
aichat -c fibonacci in js                       # Code

aichat -s                                       # REPL + New session
aichat -s session1                              # REPL + New/Reuse 'session1'

aichat --info                                   # View system info
aichat -r role1 --info                          # View role info
aichat -s session1 --info                       # View session info

cat data.toml | aichat -c to json > data.json   # Pipe stdio/stdout

aichat -f data.toml -c to json > data.json      # Attach files

aichat -f a.png -f b.png diff images            # Attach images
```

### Shell commands

Simply input what you want to do in natural language, and aichat will prompt and run the command that achieves your intent.

```
aichat -e <text>...
```

Aichat is aware of OS and shell  you are using, it will provide shell command for specific system you have. For instance, if you ask `aichat` to update your system, it will return a command based on your OS. Here's an example using macOS:

```
$ aichat -e update my system
# sudo softwareupdate -i -a
? [1]:execute [2]:explain [3]:revise [4]:cancel (1)
```

The same prompt, when used on Ubuntu, will generate a different suggestion:
```
$ aichat -e update my system
sudo apt update && sudo apt upgrade -y
? [1]:execute [2]:explain [3]:revise [4]:cancel (1)
```

### Shell integration

This is a **very handy feature**, which allows you to use `aichat` shell completions directly in your terminal, without the need to type `aichat` with prompt and arguments. This feature puts `aichat` completions directly into terminal buffer (input line), allowing for immediate editing of suggested commands.

![aichat-integration](https://github.com/sigoden/aichat/assets/4012553/873ebf23-226c-412e-a34f-c5aaa7017524)

To install shell integration, go to [./scripts/shell-integration](https://github.com/sigoden/aichat/tree/main/scripts/shell-integration) to download the script and source the script in rc file. After that restart your shell. You can invoke the completion with `alt+e` hotkey.

### Generating code

By using the `--code` or `-c` parameter, you can specifically request pure code output.

![aichat-code](https://github.com/sigoden/aichat/assets/4012553/2bbf7c8a-3822-4222-9498-693dcd683cf4)

**The `-c/--code` option ensures the extraction of code from Markdown.**

## Chat REPL

Aichat has a powerful Chat REPL.

**REPL Features:**
- **Convenient Tab Autocompletion:** Get suggestions for commands and functions while typing.
- **Customizable REPL Prompt:** Personalize the REPL interface by defining your own prompt.
- **Streamlined Keybindings:** Use familiar Emacs/Vi keybindings for efficient navigation and editing.
- **Multi-line Editing:** Create and edit multi-line inputs with ease.
- **External Editor Integration:** Open an external editor to refine the current inputs or write longer inputs.
- **History and Undo Support:** Access previously executed commands and undo any actions you make.

### `.help` - print help message

```
> .help
.help                    Show this help message
.info                    View system info
.model                   Change the current LLM
.prompt                  Make a temporary role using a prompt
.role                    Switch to a specific role
.info role               View role info
.exit role               Leave the role
.session                 Begin a chat session
.info session            View session info
.save session            Save the chat to file
.clear messages          Erase messages in the current session
.exit session            End the current session
.file                    Read files and send them as input
.set                     Adjust settings
.copy                    Copy the last response
.exit                    Exit the REPL

Type ::: to start multi-line editing, type ::: to finish it.
Press Ctrl+O to open an editor to edit line input.
Press Ctrl+C to cancel the response, Ctrl+D to exit the REPL
```

### `.info` - view information

```
> .info
model               openai:gpt-3.5-turbo
temperature         -
dry_run             false
save                true
save_session        -
highlight           true
light_theme         false
wrap                no
wrap_code           false
auto_copy           true
keybindings         emacs
prelude             -
compress_threshold  2000
config_file         /home/alice/.config/aichat/config.yaml
roles_file          /home/alice/.config/aichat/roles.yaml
messages_file       /home/alice/.config/aichat/messages.md
sessions_dir        /home/alice/.config/aichat/sessions
```

### `.model` - choose a model

```
> .model openai:gpt-4
> .model ollama:llama2
```

> You can easily enter model name using tab autocompletion.

### `.role` - let the AI play a role

Select a role:

```
> .role emoji
```

Send message with the role:

```
emoji> hello
👋
```

Leave current role:

```
emoji> .exit role

> hello
Hello there! How can I assist you today?
```

Show role info:

```
emoji> .info role
name: emoji
prompt: I want you to translate the sentences I write into emojis. I will write the sentence, and you will express it with emojis. I just want you to express it with emojis. I don't want you to reply with anything but emoji. When I need to tell you something in English, I will do it by wrapping it in curly brackets like {like this}.
temperature: null
```

Temporarily use a role to send a message.
```
> ::: .role emoji
hello world
:::
👋🌍

> 
```

### `.session` - context-aware conversation

By default, aichat behaves in a one-off request/response manner.

You should run aichat with `-s/--session` or use the `.session` command to start a session.


```
> .session

temp) 1 to 5, odd only                                                                    0
1, 3, 5

temp) to 7                                                                        19(0.46%)
1, 3, 5, 7

temp) .exit session                                                               42(1.03%)
? Save session? (y/N)  

```

The prompt on the right side is about the current usage of tokens and the proportion of tokens used, 
compared to the maximum number of tokens allowed by the model.


### `.prompt` - make a temporary role using a prompt

There are situations where setting a system message is necessary, but modifying the `roles.yaml` file is undesirable.
To address this, we leverage the `.prompt` to create a temporary role specifically for this purpose.

```
> .prompt write unit tests for the rust functions

%%>
```

### `.file` - include files with the message

```
Usage: .file <file>... [-- text...]

.file message.txt
.file config.yaml -- convert to toml
.file a.jpg b.jpg -- What’s in these images?
.file https://ibb.co/a.png https://ibb.co/b.png -- what is the difference?
```

> Only the current model that supports vision can process images submitted through `.file` command.

### `.set` - modify the configuration temporarily

```
.set temperature 1.2
.set compress_threshold 1000
.set dry_run true
.set highlight false
.set save false
.set save_session true
.set auto_copy true
```

### Roles

We can define a batch of roles in `roles.yaml`.

> Get `roles.yaml` path with command `aichat --info` or repl command `.info`.

For example, we can define a role:

```yaml
- name: shell
  prompt: >
    I want you to act as a Linux shell expert.
    I want you to answer only with bash code.
    Do not provide explanations.
```

Let LLM answer questions in the role of a Linux shell expert.

```
> .role shell

shell>  extract encrypted zipfile app.zip to /tmp/app
mkdir /tmp/app
unzip -P PASSWORD app.zip -d /tmp/app
```

For more details about roles, please visit [Role Guide](https://github.com/sigoden/aichat/wiki/Role-Guide).

## License

Copyright (c) 2023-2024 aichat-developers.

Aichat is made available under the terms of either the MIT License or the Apache License 2.0, at your option.

See the LICENSE-APACHE and LICENSE-MIT files for license details.

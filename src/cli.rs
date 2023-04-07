use std::{
    error::Error,
    fmt::Display,
    fs::{self, File},
    io::{self, prelude::*, BufReader},
    path::{Path, PathBuf},
    process::Command,
};

use async_openai::{
    types::{ChatCompletionRequestMessageArgs, Role},
    Client,
};
use clap::Parser;
use serde_json::Value;

use crate::OpenAIChat;

/// Automatically fixes bugs in your code
#[derive(Parser, Debug)]
#[command(author, version, about)]
struct Args {
    /// Path to file to fix
    filepath: PathBuf,
}

#[derive(Debug)]
pub enum FixError {
    UnrecognizableFileExtension,
}

impl Display for FixError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{self:#?}")
    }
}

impl Error for FixError {}

pub async fn cli(api_key: String) -> Result<(), Box<dyn Error>> {
    let args = Args::parse();
    let client = Client::new().with_api_key(api_key);

    let file = fs::read_to_string(args.filepath.as_path())?;

    let (mut command, lang) = match args.filepath.extension().unwrap().to_str().unwrap() {
        "py" => (Command::new("python"), "python"),
        _ => return Err(Box::new(FixError::UnrecognizableFileExtension)),
    };
    let mut chat = OpenAIChat::new(client, ChatCompletionRequestMessageArgs::default()
                                   .role(Role::System)
                                   .content(format!(r#"You are a brilliant {lang} programmer that is one of the best in the world at finding and fixing bugs in code. You will be given a program that has bug(s) in it, along with the stack traces, and your job is to fix the bug(s) and return your changes in a very specific format specified below. 
Please return a JSON object containing the suggested changes in a format similar to the git diff system, showing whether a line is added, removed, or edited for each file.
    For example, the JSON output should look like:
    {{
    "intent": "This should be what you think the program SHOULD do.",
    "explanation": "Explanation of what went wrong and the changes being made",
    "files": [
        {{
            "file_name": "file_name.py",
            "changes": [
                {{
                "action": "edit",
                "line_number": 2,
                "new_line": "print('hello world')",
                }},
                {{
                "action": "add",
                "line_number": 3,
                "new_line": "hello="world"\nprint(hello)\nprint(1 + 2)",
                }},
                {{
                "action": "remove",
                "line_number": 4,
                "new_line": "",
                }},
            ]
        }},
    ]
    }}
    In the 'action' field, please use "add" for adding a line, this will put the line after the line number, "remove" for removing a line, and "edit" for editing a line. Please provide the suggested changes in this format. The code has line numbers prepended to each line in the format "1:print('hello world')", so you can use that to determine the line number on which to make a change. Edits are applied in reverse line order so that the line numbers don't change as you edit the code.PLAY VERY CLOSE ATTENTION TO INDENTATION AND WHITESPACE, THIS IS PYTHON AFTER ALL! DO NOT DEVIATE FROM THE FORMAT IT MUST BE ABLE TO BE PARSED BY ME! You will be penalized if you do. ONLY RETURN JSON, DON'T EXPLAIN YOURSELF UNLESS IN THE EXPLANATION FIELD. DON'T INCLUDE MARKDOWN BACKTICKS OR ANYTHING LIKE THAT, JUST THE JSON."#))
                                   .build()?);

    command.arg(args.filepath);

    while let Err(e) = run_code(&mut command) {
        let out = chat
            .complete(format!(
                "I get this error\n{}\nWhen I run this code\n{}\n",
                e, file
            ))
            .await
            .unwrap();

        println!("{}", out.content);
        let json: Value = match serde_json::from_str(&out.content) {
            Ok(x) => x,
            Err(_) => continue,
        };
        edit_files(json);
    }
    Ok(())
}

fn run_code(command: &mut Command) -> Result<(), String> {
    let output = command.output().unwrap();
    if output.status.code().unwrap() == 1 {
        let error = String::from_utf8(output.stderr).unwrap();
        println!("{:#?}", error);
        Err(error)
    } else {
        println!("code works!\n{}", String::from_utf8(output.stdout).unwrap());
        Ok(())
    }
}

fn edit_files(json: Value) {
    let json = json["files"].as_array().unwrap()[0].to_owned();

    let filepath = json["file_name"].to_string();
    let filepath = filepath.trim_matches('"');

    let changes = match json["changes"].as_array() {
        Some(x) => x,
        None => return,
    };

    println!("{filepath}");

    let file = fs::File::open(filepath).unwrap();
    let mut lines: Vec<String> = io::BufReader::new(file)
        .lines()
        .map(|s| {
            println!("{s:#?}");
            s.unwrap()
        })
        .collect();

    println!("{lines:#?}");

    for change in changes {
        let action = change["action"].as_str().unwrap();
        let line_num = change["line_number"].as_u64().unwrap() as usize;

        let new_line = change["new_line"].as_str().unwrap();

        match action {
            "edit" => lines[line_num] = new_line.to_owned(),
            "remove" => lines[line_num] = "".to_owned(),
            "add" => lines.insert(line_num as usize, new_line.to_owned()),
            _ => {}
        }
    }

    fs::write(filepath, lines.join("\n")).unwrap();
}

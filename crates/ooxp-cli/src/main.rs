//! ooxp 命令行：从文件或 stdin 读领域 JSON。

use std::io::{self, Read};
use std::path::PathBuf;
use std::process::ExitCode;

use clap::Parser;
use ooxp::{run, written_json, RunOutput};

#[derive(Parser, Debug)]
#[command(name = "ooxp", about = "用领域 JSON 读写 OOXML 文件")]
struct Cli {
    /// 领域 JSON 路径。`-` 或省略时从 stdin 读。
    #[arg(long)]
    input: Option<String>,
    /// create 缺 path、或 apply 缺 output 时的落盘路径。
    #[arg(long)]
    output: Option<PathBuf>,
    /// 把 stdout 的 JSON 缩进打印。
    #[arg(long)]
    pretty: bool,
}

fn main() -> ExitCode {
    let cli = Cli::parse();
    match real_main(cli) {
        Ok(()) => ExitCode::SUCCESS,
        Err(err) => {
            let payload = err.to_json();
            eprintln!("{}", serde_json::to_string(&payload).unwrap());
            ExitCode::from(err.exit_code() as u8)
        }
    }
}

fn real_main(cli: Cli) -> Result<(), ooxp::Error> {
    let json_text = read_input(cli.input.as_deref())?;
    let out = run(&json_text, cli.output.as_deref())?;
    match out {
        RunOutput::Inspect(tree) => {
            let text = if cli.pretty {
                serde_json::to_string_pretty(&tree).unwrap()
            } else {
                serde_json::to_string(&tree).unwrap()
            };
            println!("{text}");
        }
        RunOutput::Written { path, ops_applied } => {
            println!("{}", written_json(&path, ops_applied, cli.pretty));
        }
    }
    Ok(())
}

fn read_input(input: Option<&str>) -> Result<String, ooxp::Error> {
    match input {
        None | Some("-") => {
            let mut buf = String::new();
            io::stdin()
                .read_to_string(&mut buf)
                .map_err(ooxp::Error::from)?;
            Ok(buf)
        }
        Some(path) => std::fs::read_to_string(path).map_err(ooxp::Error::from),
    }
}

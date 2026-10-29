//! ooxp 命令行：从文件或 stdin 读领域 JSON。`mcp` 改读 JSON-RPC。

mod mcp;

use std::io::{self, Read};
use std::path::PathBuf;
use std::process::ExitCode;

use clap::Parser;
use ooxp::{io_path, peek_lang, run, written_json, Error, Lang, RunOutput};

#[derive(Parser, Debug)]
#[command(name = "ooxp", about = "用领域 JSON 读写 OOXML 文件")]
struct Cli {
    /// 领域 JSON 路径。`-` 或省略时从 stdin 读。
    #[arg(long)]
    input: Option<String>,
    /// create 缺 path、或 apply / exportMedia 缺 output 时的落盘路径。
    #[arg(long)]
    output: Option<PathBuf>,
    /// 错误文案语言。请求里的 locale 优先。
    #[arg(long)]
    lang: Option<String>,
    /// 把 stdout 的 JSON 缩进打印。
    #[arg(long)]
    pretty: bool,
}

fn main() -> ExitCode {
    let args: Vec<_> = std::env::args_os().skip(1).collect();
    if args.first().and_then(|arg| arg.to_str()) == Some("mcp") {
        return match mcp::parse_lang(&args) {
            Ok(lang) => mcp::serve(lang),
            Err(message) => {
                eprintln!("{message}");
                ExitCode::from(1)
            }
        };
    }
    let cli = Cli::parse();
    let fallback = Lang::resolve(
        None,
        cli.lang.as_deref(),
        std::env::var("OOXP_LANG").ok().as_deref(),
    );
    match real_main(&cli) {
        Ok(()) => ExitCode::SUCCESS,
        Err((err, lang)) => {
            let lang = lang.unwrap_or(fallback);
            let payload = err.to_json(lang);
            eprintln!("{}", serde_json::to_string(&payload).unwrap());
            ExitCode::from(err.exit_code() as u8)
        }
    }
}

fn real_main(cli: &Cli) -> Result<(), (Error, Option<Lang>)> {
    let json_text = read_input(cli.input.as_deref()).map_err(|e| (e, None))?;
    let lang = Lang::resolve(
        peek_lang(&json_text),
        cli.lang.as_deref(),
        std::env::var("OOXP_LANG").ok().as_deref(),
    );
    let out = run(&json_text, cli.output.as_deref()).map_err(|e| (e, Some(lang)))?;
    match out {
        RunOutput::Inspect(tree) | RunOutput::Help(tree) => {
            let text = if cli.pretty {
                serde_json::to_string_pretty(&tree).unwrap()
            } else {
                serde_json::to_string(&tree).unwrap()
            };
            println!("{text}");
        }
        RunOutput::Written {
            path,
            ops_applied,
            content_type,
        } => {
            println!(
                "{}",
                written_json(&path, ops_applied, content_type.as_deref(), cli.pretty)
            );
        }
    }
    Ok(())
}

fn read_input(input: Option<&str>) -> Result<String, Error> {
    match input {
        None | Some("-") => {
            let mut buf = String::new();
            io::stdin().read_to_string(&mut buf).map_err(Error::from)?;
            Ok(buf)
        }
        // 带上路径，否则读不到文件时只剩一句 I/O failed，看不出是哪个文件。
        Some(path) => std::fs::read_to_string(path).map_err(|e| io_path(e, PathBuf::from(path))),
    }
}

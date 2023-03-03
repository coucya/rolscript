#![allow(dead_code, unused_imports, unused_variables)]

use std::fs;
use std::path::Path;

use clap;
use clap::Parser;

use rolscript as rs;
use rolscript::Error as RError;
use rolscript::*;

mod functons;
mod loader;

use loader::StdLoader;

fn load_file(path: &Path) -> String {
    String::from_utf8(fs::read(path).unwrap()).unwrap()
}

static mut STD_LOADER: StdLoader = StdLoader;

#[derive(Parser)]
#[command(name = "rols")]
#[command(version = "0.1.0")]
#[command(about = "rolscript interpreter.", long_about = None)]
struct Args {
    file: Option<String>,

    #[arg(short, long)]
    eval: Option<String>,
}

fn eval_file(file_path: &Path, file_source: &str) -> Result<(), RError> {
    let module_name = RString::new(file_path.to_str().unwrap())?;
    let input_module = RModule::new(module_name, None)?;
    rs::eval_with_module(&input_module, file_source)?;
    Ok(())
}

fn print_error(err: RError) {
    match err {
        Error::Parse(pe) => {
            println!("error: {}", pe.msg().as_str());
            println!("   at: {}:{}", pe.pos().line, pe.pos().column);
        }
        Error::Runtime(re) => {
            if re.is_type(rs::string_type()) {
                let s = unsafe { re.cast_ref::<RString>() };
                println!("error: {}", s.as_str());
            } else {
                println!("error: {}", rolscript::value_str(&re).unwrap().as_str());
            }
        }
        e => {
            println!("{:?}", e)
        }
    }
}

fn print_value(value: RValue) {
    if let Ok(s) = rolscript::value_str(&value) {
        println!("{}", s.as_str());
    } else {
        println!("{:?}", &value);
    }
}

fn main() {
    let args = Args::parse();

    let allocator = rolscript::default_allocator();
    let loader = unsafe { &mut STD_LOADER };

    initialize(allocator, loader).unwrap();

    if let Some(file) = &args.file {
        functons::add_all().unwrap();

        let file_path = match Path::new(file).canonicalize() {
            Ok(path) => path,
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => {
                println!("{} not found", file);
                return;
            }
            Err(e) => {
                println!("can't eval {}", file);
                return;
            }
        };
        let source_string = load_file(&file_path);

        let ret = eval_file(&file_path, &source_string);
        match ret {
            Ok(()) => (),
            Err(e) => print_error(e),
        }
    } else if let Some(eval) = &args.eval {
        functons::add_all().unwrap();

        let ret = rs::eval(&eval);
        match ret {
            Ok(v) => print_value(v),
            Err(e) => print_error(e),
        }
    } else {
        functons::add_all().unwrap();

        let input_module_name = RString::new("<input>").unwrap();
        let input_module = RModule::new(input_module_name, None).unwrap();

        use std::io::{Stdin, Stdout, Write};
        let mut stdout = std::io::stdout();
        let stdin = std::io::stdin();

        loop {
            print!(">> ");
            stdout.flush().unwrap();

            let mut buf = String::new();
            stdin.read_line(&mut buf).unwrap();

            // TODO: 真正实现exit。
            if buf.trim() == "exit()" || buf.trim() == "exit();" {
                break;
            }

            let ret = rs::eval_with_module(&input_module, &buf);
            match ret {
                Ok(v) => print_value(v),
                Err(e) => print_error(e),
            }
        }
    }

    finalize();
}

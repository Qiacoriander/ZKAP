use clap::Parser;
use ir_generator::generator::generate;
use program_structure::ast::{Definition, Expression, AST};
use program_structure::error_definition::Report;
use std::collections::HashSet;
use std::fs;
use std::{env, fs::canonicalize, path::PathBuf};

#[derive(Parser, Debug)]
#[command(
    author = "Hongbo Wen, whbjzzwjxq@gmail.com",
    version = env!("CARGO_PKG_VERSION"),
    about = "Compile circom code to LLVM IR.",
    long_about = None
)]
struct Args {
    /// Path of the input file or folder.
    #[arg(short, long)]
    input: String,

    /// Path of the output folder, default is the cwd.
    #[arg(short, long)]
    output: Option<String>,

    /// If provided, systematically rewrite all of the paths about circomlib.
    #[arg(short, long)]
    rewrite_circomlib: Option<String>,

    /// The static size of array.
    #[arg(short, long, default_value_t = 256)]
    arraysize: u32,

    /// The compilation mode, instantiation or abstraction.
    #[arg(long, default_value_t = false)]
    instantiation: bool,

    /// Generate summary or not.
    #[arg(long, default_value_t = false)]
    summarize: bool,
}

fn main() {
    let args = Args::parse();
    let input = PathBuf::from(args.input);
    let input_paths: Vec<PathBuf>;
    if input.is_dir() {
        // 这里没有递归地处理子目录
        let files_res = fs::read_dir(input);
        match files_res {
            Ok(files) => {
                input_paths = files
                    .map(|i| i.unwrap().path())
                    .filter(|p| {
                        let ext_op = p.extension();
                        match ext_op {
                            Some(ext) => ext == "circom",
                            None => false,
                        }
                    })
                    .collect();
            }
            Err(err) => {
                println!("Error: {}", err);
                return;
            }
        }
    } else {
        input_paths = vec![input];
    }

    let output = match args.output {
        None => env::current_dir().unwrap(),
        Some(o) => PathBuf::from(o),
    };
    if !output.is_dir() {
        println!(
            "Error: output is NOT a directory or doesn't exist, current is : {}",
            output.to_string_lossy()
        );
        return;
    }

    let circomlib_path = match args.rewrite_circomlib {
        None => None,
        Some(rc) => Some(PathBuf::from(rc)),
    };

    for input_path in input_paths {
        let input_filename = input_path.file_name().unwrap();
        let output_path = output.join(input_filename).with_extension("ll");
        let output_summary_path = output.join(input_filename).with_extension("json");
        println!("Compiling: {}", input_path.to_string_lossy());
        println!("Output: {}", output_path.to_string_lossy());
        // Parsing
        let main_expr: Option<Expression>;
        // 解析获取每个文件的AST，每个文件对应一个AST，但一个AST中有多个Definition
        let entry_ast = parser::run_parser(input_path.clone(), Vec::new());
        match entry_ast {
            Ok(ast) => {
                match ast.main_component {
                    Some((_, expr)) => {
                        main_expr = Some(expr);
                    }
                    None => {
                        main_expr = None;
                    }
                }
            }
            Err((file_library, report_collection)) => {
                Report::print_reports(&report_collection, &file_library);
                unreachable!();
            }
        }
        // 不但
        let mut todo_paths: Vec<PathBuf> = vec![input_path.clone()];
        let mut done_paths: HashSet<PathBuf> = HashSet::new(); // 确保每个文件只被解析一次
        let mut asts: Vec<AST> = vec![];
        while todo_paths.len() > 0 {
            let input_path = todo_paths.pop().unwrap();
            let mut working_dir = input_path.clone();
            working_dir.pop();
            match parser::run_parser(input_path, Vec::new()) {
                Ok(ast) => {
                    for i in &ast.includes {
                        let include_path = match &circomlib_path {
                            None => working_dir.join(i),    // 简单情况，直接在工作目录下找
                            Some(s) => {          // 复杂情况，需要处理circomlib路径映射
                                let include_path = working_dir.join(i);
                                let mut actual_path = PathBuf::new();
                                let mut has_circomlib = false;
                                for p in &include_path {
                                    if has_circomlib {
                                        actual_path.push(p);
                                    }
                                    if p.to_string_lossy() == "circomlib" {
                                        has_circomlib = true;
                                    }
                                }
                                if has_circomlib {
                                    s.join(actual_path) // 映射到外部指定的circomlib路径
                                } else {
                                    include_path        // 普通路径，保持不变
                                }
                            }
                        };
                        let abs_path = match canonicalize(&include_path) {
                            Ok(p) => p,
                            Err(_) => {
                                println!("Path is not found: {}", include_path.to_string_lossy());
                                unreachable!();
                            }
                        };
                        if done_paths.contains(&abs_path) {
                            continue;
                        }
                        todo_paths.push(include_path);
                        done_paths.insert(abs_path);
                    }
                    // 把当前文件所有的include都加入到todo_paths中了，这个文件本身就算处理完了，加入asts中
                    asts.push(ast);
                }
                Err((file_library, report_collection)) => {
                    Report::print_reports(&report_collection, &file_library);
                }
            }
        }

        // Compiling
        let mut definition_names = HashSet::new();
        let mut definitions = Vec::new();
        for ast in &asts {
            for def in ast.get_definitions() {
                match def {
                    Definition::Template { name, .. } => {
                        if definition_names.contains(name) {
                            continue;
                        }
                        definition_names.insert(name.clone());
                        definitions.push(def);
                    }
                    Definition::Function { name, .. } => {
                        if definition_names.contains(name) {
                            continue;
                        }
                        definition_names.insert(name.clone());
                        definitions.push(def);
                    }
                }
            }
        }
        generate(
            args.instantiation,
            args.arraysize,
            main_expr,
            definitions,
            &input_path,
            &output_path,
            &output_summary_path,
            args.summarize,
        );
    }
}

use comment_check::check_source;
use std::path::{Path, PathBuf};
use std::process::ExitCode;
use std::{env, fs, io};

fn rust_files(dir: &Path, out: &mut Vec<PathBuf>) -> io::Result<()> {
    let mut entries: Vec<_> = fs::read_dir(dir)?.collect::<Result<_, _>>()?;
    entries.sort_by_key(|entry| entry.path());
    for entry in entries {
        let path = entry.path();
        if path.is_dir() {
            if path
                .file_name()
                .is_some_and(|name| name != "target" && name != ".git")
            {
                rust_files(&path, out)?;
            }
        } else if path.extension().is_some_and(|ext| ext == "rs") {
            out.push(path);
        }
    }
    Ok(())
}

fn main() -> ExitCode {
    let roots: Vec<PathBuf> = env::args_os().skip(1).map(PathBuf::from).collect();
    let roots = if roots.is_empty() {
        vec![PathBuf::from(".")]
    } else {
        roots
    };
    let mut files = Vec::new();
    for root in &roots {
        if let Err(error) = rust_files(root, &mut files) {
            eprintln!("error: {}: {error}", root.display());
            return ExitCode::FAILURE;
        }
    }
    let mut failed = false;
    for file in files {
        let source = match fs::read_to_string(&file) {
            Ok(source) => source,
            Err(error) => {
                eprintln!("error: {}: {error}", file.display());
                return ExitCode::FAILURE;
            }
        };
        for violation in check_source(&source) {
            eprintln!("{}: {violation}", file.display());
            failed = true;
        }
    }
    if failed {
        ExitCode::FAILURE
    } else {
        ExitCode::SUCCESS
    }
}

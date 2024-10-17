use serde::Deserialize;
use std::{
    env::temp_dir,
    fmt,
    fs::{self, create_dir},
    ops::ControlFlow,
    path::PathBuf,
    process::Command,
};
use tempfile::{tempdir, tempdir_in, TempDir};

use clap::{Args, Parser, ValueEnum};

#[derive(Debug, Deserialize)]
struct Instruction {
    url: String,
    name: String,
    package: String,
    rev: String,
    before_action: Option<String>,
}

#[derive(Debug, Deserialize)]
struct Test {
    url: String,
    name: String,
    rev: String,
}

#[derive(Debug, Deserialize)]
struct Config {
    test: Test,
    instructions: Vec<Instruction>,
}

#[derive(Clone, ValueEnum)]
enum Cmd {
    Check,
    Test,
}

impl fmt::Display for Cmd {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self {
            Cmd::Check => write!(f, "check"),
            Cmd::Test => write!(f, "test"),
        }
    }
}

#[derive(Clone, Parser)]
struct Cli {
    config: PathBuf,
    #[arg(value_enum, default_value_t = Cmd::Check)]
    cmd: Cmd,
    #[arg(long)]
    root_dir: Option<PathBuf>,
}

fn tmp_dir(root_dir: &Option<PathBuf>) -> TempDir {
    if let Some(ref root_dir) = root_dir {
        tempdir_in(root_dir).unwrap()
    } else {
        tempdir().unwrap()
    }
}

fn main() {
    let args = Cli::parse();
    // Load and parse the TOML file
    let toml_content = fs::read_to_string(args.config).unwrap();
    let config: Config = toml::from_str(&toml_content).unwrap();

    if let Some(root_dir) = &args.root_dir {
        if !fs::exists(root_dir).unwrap() {
            create_dir(root_dir).unwrap();
        }
    }

    let test = config.test;
    let test_tmp_dir = tmp_dir(&args.root_dir);
    println!("[-] Cloning into {:?}", test_tmp_dir);
    let output = Command::new("git")
        .args(["clone", &test.url, "--branch", &test.rev])
        .current_dir(&test_tmp_dir)
        .output()
        .unwrap();
    if !output.status.success() {
        let stdout = String::from_utf8(output.stdout).unwrap();
        println!("stdout: {}", stdout);
        let stderr = String::from_utf8(output.stderr).unwrap();
        println!("stderr: {}", stderr);
        return;
    }
    for Instruction {
        url,
        name,
        package,
        rev,
        before_action,
    } in config.instructions
    {
        let tmp_dir = tmp_dir(&args.root_dir);
        // persist
        let tmp_dir = tmp_dir.into_path();
        // let tmp_dir = tmp_dir.path();
        println!("[-] Cloning into {:?}", tmp_dir);
        let output = Command::new("git")
            .args(["clone", &url, "--branch", &rev])
            .current_dir(&tmp_dir)
            .output()
            .unwrap();
        if !output.status.success() {
            let stdout = String::from_utf8(output.stdout).unwrap();
            println!("stdout: {}", stdout);
            let stderr = String::from_utf8(output.stderr).unwrap();
            println!("stderr: {}", stderr);
            break;
        }

        let proj_dir = tmp_dir.join(name);

        if let Some(before_action) = &before_action {
            run_extra_cmd(&before_action, &proj_dir);
        }

        println!("[-] Evaluating {:?}", proj_dir);
        let output = Command::new("cargo")
            .args([args.cmd.to_string()])
            .current_dir(&proj_dir)
            .output()
            .unwrap();
        if !output.status.success() {
            let stdout = String::from_utf8(output.stdout).unwrap();
            println!("stdout: {}", stdout);
            let stderr = String::from_utf8(output.stderr).unwrap();
            println!("stderr: {}", stderr);
            break;
        }

        let test_path = test_tmp_dir.path().join(&test.name);
        let test_path = test_path.as_os_str().to_str().unwrap();
        println!(
            "[-] Modified dep {} in {:?} with {:?} with package {}",
            test.name, proj_dir, test_path, package,
        );
        let output = Command::new("cargo")
            .args(["add", &test.name, "--path", test_path, "-p", &package])
            .current_dir(&proj_dir)
            .output()
            .unwrap();
        if !output.status.success() {
            let stdout = String::from_utf8(output.stdout).unwrap();
            println!("stdout: {}", stdout);
            let stderr = String::from_utf8(output.stderr).unwrap();
            println!("stderr: {}", stderr);
            break;
        }

        if let Some(before_action) = &before_action {
            run_extra_cmd(&before_action, &proj_dir);
        }

        println!("[-] Checking Modified {:?}", proj_dir);
        let output = Command::new("cargo")
            .args(["check"])
            .current_dir(&proj_dir)
            .output()
            .unwrap();
        if !output.status.success() {
            let stdout = String::from_utf8(output.stdout).unwrap();
            println!("stdout: {}", stdout);
            let stderr = String::from_utf8(output.stderr).unwrap();
            println!("stderr: {}", stderr);
            break;
        }
    }

    // persist
    let _ = test_tmp_dir.into_path();
}

fn run_extra_cmd(test_extra_cmd: &str, proj_dir: &PathBuf) {
    if test_extra_cmd != "" {
        println!("[-] running extra cmd: {test_extra_cmd}");
        let split: Vec<&str> = test_extra_cmd.split_whitespace().collect();
        let output = Command::new(&split[0])
            .args(&split[1..])
            .current_dir(proj_dir)
            .output()
            .unwrap();
        if !output.status.success() {
            let stdout = String::from_utf8(output.stdout).unwrap();
            println!("stdout: {}", stdout);
            let stderr = String::from_utf8(output.stderr).unwrap();
            println!("stderr: {}", stderr);
            return;
        }
    }
}

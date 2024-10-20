use serde::Deserialize;
use std::{
    env::temp_dir,
    fmt,
    fs::{self, create_dir},
    ops::ControlFlow,
    path::PathBuf,
    process::{Command, Output, Stdio},
};
use tempfile::{tempdir, tempdir_in, TempDir};

use clap::{Args, Parser, ValueEnum};

mod github;

#[derive(Clone, Debug, Deserialize)]
struct Instruction {
    url: String,
    name: String,
    package: Option<String>,
    rev: Option<String>,
    before_action: Option<String>,
}

#[derive(Clone, Debug, Deserialize)]
struct Test {
    url: String,
    name: String,
    rev: String,
}

#[derive(Clone, Debug, Deserialize)]
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
    /// Config
    config: PathBuf,
    #[arg(value_enum, default_value_t = Cmd::Check)]
    /// Command to run
    cmd: Cmd,
    /// Root directory, tmp if not given
    #[arg(long)]
    root_dir: Option<PathBuf>,
    /// Github Dependents Json
    #[arg(long)]
    from_github_dependents_info: Option<PathBuf>,
    /// Don't exit on single failure
    #[arg(long)]
    no_exit_on_error: bool,
    /// Don't emit stdout while running commands
    #[arg(long)]
    no_stdout: bool,
}

fn tmp_dir(root_dir: &Option<PathBuf>) -> TempDir {
    if let Some(ref root_dir) = root_dir {
        tempdir_in(root_dir).unwrap()
    } else {
        tempdir().unwrap()
    }
}

use std::io::{self, BufRead, BufReader};

fn run(cli: &Cli, command: &mut Command) -> io::Result<bool> {
    if !cli.no_stdout {
        command.stdout(Stdio::piped());

        let mut child = command.spawn()?;

        // Ensure the child's stdout can be captured
        if let Some(stdout) = child.stdout.take() {
            // Create a buffered reader to process the stdout line-by-line
            let reader = BufReader::new(stdout);

            // Read lines from the command's stdout as they are produced
            for line in reader.lines() {
                // Print each line to the current program's stdout
                println!("{}", line?);
            }
        }

        // Wait for the child process to finish
        let status = child.wait()?;
        let success = status.success();
        if success {
            println!("[-] success!");
        }
        Ok(success)
    } else {
        let output = command.output().unwrap();
        let success = output.status.success();
        if !success {
            let stdout = String::from_utf8(output.stdout).unwrap();
            println!("stdout: {}", stdout);
            let stderr = String::from_utf8(output.stderr).unwrap();
            println!("stderr: {}", stderr);
        }
        if success {
            println!("[-] success!");
        }
        Ok(success)
    }
}

fn main() {
    let args = Cli::parse();

    let toml_content = fs::read_to_string(&args.config).unwrap();
    let mut config: Config = toml::from_str(&toml_content).unwrap();

    if let Some(ref github) = args.from_github_dependents_info {
        let github = fs::read_to_string(github).unwrap();
        let repos = github::dependents_info(github).unwrap();
        for g in repos.all_public_dependent_repos {
            // omit forks
            if g.repo_name != "deku" {
                config.instructions.push(Instruction {
                    url: format!("https://github.com/{}", g.name),
                    name: g.repo_name,
                    package: None,
                    rev: None,
                    before_action: None,
                })
            }
        }
    }
    if let Some(root_dir) = &args.root_dir {
        if !fs::exists(root_dir).unwrap() {
            create_dir(root_dir).unwrap();
        }
    }

    let test = config.test;
    let test_tmp_dir = tmp_dir(&args.root_dir);
    println!("[-] Cloning into {:?}", test_tmp_dir);
    let mut cmd = Command::new("git");
    cmd.args(["clone", &test.url, "--branch", &test.rev])
        .current_dir(&test_tmp_dir);
    if !run(&args, &mut cmd).unwrap() && !args.no_exit_on_error {
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

        let mut git_args = vec!["clone".to_string(), url.to_string()];
        if let Some(rev) = rev {
            git_args.append(&mut vec!["--branch".to_string(), rev.clone()]);
        }

        let mut cmd = Command::new("git");
        cmd.args(git_args).current_dir(&tmp_dir);
        if !run(&args, &mut cmd).unwrap() && !args.no_exit_on_error {
            break;
        }

        let proj_dir = tmp_dir.join(name);

        if let Some(before_action) = &before_action {
            run_extra_cmd(&args, &before_action, &proj_dir);
        }

        println!("[-] Evaluating {:?}", proj_dir);
        let mut cmd = Command::new("cargo");
        cmd.args([args.cmd.to_string()]).current_dir(&proj_dir);
        if !run(&args, &mut cmd).unwrap() && !args.no_exit_on_error {
            break;
        }

        let test_path = test_tmp_dir.path().join(test.clone().name.clone());
        let test_path = test_path.as_os_str().to_str().unwrap();
        println!(
            "[-] Modified dep {} in {:?} with {:?}",
            test.clone().name,
            proj_dir,
            test_path,
        );

        let mut cargo_add_args = vec![
            "add".to_string(),
            test.clone().name,
            "--path".to_string(),
            test_path.to_string(),
        ];
        if let Some(package) = package {
            println!("[-] with package: {}", package);
            cargo_add_args.append(&mut vec!["-p".to_string(), package.to_string()]);
        }
        let mut cmd = Command::new("cargo");
        cmd.args(cargo_add_args).current_dir(&proj_dir);
        if !run(&args, &mut cmd).unwrap() && !args.no_exit_on_error {
            break;
        }
        if let Some(before_action) = &before_action {
            run_extra_cmd(&args, &before_action, &proj_dir);
        }

        println!("[-] Checking Modified {:?}", proj_dir);
        let mut cmd = Command::new("cargo");
        cmd.args(["check"]).current_dir(&proj_dir);
        if !run(&args, &mut cmd).unwrap() && !args.no_exit_on_error {
            break;
        }
    }

    // persist
    let _ = test_tmp_dir.into_path();
}

fn run_extra_cmd(args: &Cli, test_extra_cmd: &str, proj_dir: &PathBuf) {
    if test_extra_cmd != "" {
        println!("[-] running extra cmd: {test_extra_cmd}");
        let split: Vec<&str> = test_extra_cmd.split_whitespace().collect();
        let mut cmd = Command::new(&split[0]);
        cmd.args(&split[1..]).current_dir(proj_dir);
        if !run(&args, &mut cmd).unwrap() && !args.no_exit_on_error {
            return;
        }
    }
}

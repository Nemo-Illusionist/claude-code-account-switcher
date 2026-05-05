pub fn run(shell: &str) {
    let bin = std::env::current_exe()
        .expect("Cannot determine binary path")
        .to_str()
        .expect("Invalid binary path")
        .to_string();

    let template = match shell {
        "zsh" => include_str!("../../shell/init.zsh"),
        "bash" => include_str!("../../shell/init.bash"),
        "pwsh" | "powershell" => include_str!("../../shell/init.ps1"),
        other => {
            eprintln!("Unsupported shell: {}. Use: zsh, bash, pwsh", other);
            std::process::exit(1);
        }
    };

    print!("{}", template.replace("__CLAUDE_ACC_BIN__", &bin));
}

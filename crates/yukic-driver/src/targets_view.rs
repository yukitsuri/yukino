use owo_colors::OwoColorize;
use yukic_target::spec::{Architecture, Environment, ObjectFormat, OperatingSystem};
use yukic_target::TargetTriple;

fn arch_str(arch: Architecture) -> &'static str {
    match arch {
        Architecture::X86_64 => "x86_64",
        Architecture::Aarch64 => "aarch64",
        Architecture::Armv7 => "armv7",
    }
}

fn os_str(os: OperatingSystem) -> &'static str {
    match os {
        OperatingSystem::Linux => "linux",
        OperatingSystem::Windows => "windows",
        OperatingSystem::Android => "android",
    }
}

fn env_str(env: Environment) -> &'static str {
    match env {
        Environment::Gnu => "gnu",
        Environment::Musl => "musl",
        Environment::Bionic => "bionic",
        Environment::Msvc => "msvc",
    }
}

fn format_str(fmt: ObjectFormat) -> &'static str {
    match fmt {
        ObjectFormat::Elf => "elf",
        ObjectFormat::Coff => "coff",
    }
}

fn format_target_row(target: TargetTriple, host: TargetTriple, colored: bool) -> String {
    let spec = target.spec();
    let is_host = target == host;
    let default_str = if is_host { "[host]" } else { "" };

    let triple_pad = format!("{:<32}", target.as_str());
    let arch_pad = format!("{:<9}", arch_str(spec.arch));
    let os_pad = format!("{:<9}", os_str(spec.os));
    let env_pad = format!("{:<8}", env_str(spec.env));
    let fmt_pad = format!("{:<8}", format_str(spec.object_format));

    if !colored {
        if is_host {
            return format!("{triple_pad} {arch_pad} {os_pad} {env_pad} {fmt_pad} {default_str}");
        }
        return format!("{triple_pad} {arch_pad} {os_pad} {env_pad} {fmt_pad}");
    }

    let triple_col = if is_host { triple_pad.green().bold().to_string() } else { triple_pad.white().to_string() };
    let arch_col = arch_pad.yellow().to_string();
    let os_col = os_pad.cyan().to_string();
    let env_col = env_pad.magenta().to_string();
    let fmt_col = if is_host { fmt_pad.blue().bold().to_string() } else { fmt_pad.dimmed().to_string() };

    if is_host {
        let def_col = default_str.green().bold().to_string();
        format!("{triple_col} {arch_col} {os_col} {env_col} {fmt_col} {def_col}")
    } else {
        format!("{triple_col} {arch_col} {os_col} {env_col} {fmt_col}")
    }
}

pub fn targets_output(colored: bool) -> String {
    let host = TargetTriple::host();
    let all = TargetTriple::all();
    let mut lines = Vec::with_capacity(all.len() + 4);

    if colored {
        lines.push(format!(
            "{} {}",
            "Yukino Supported Target Triples".cyan().bold(),
            format!("({} available)", all.len()).dimmed()
        ));
        lines.push(format!("{}: {}", "Host".dimmed(), host.as_str().yellow().bold()));
    } else {
        lines.push(format!("Yukino Supported Target Triples ({} available)", all.len()));
        lines.push(format!("Host: {}", host.as_str()));
    }

    lines.push(String::new());

    let h_triple = format!("{:<32}", "Target Triple");
    let h_arch = format!("{:<9}", "Arch");
    let h_os = format!("{:<9}", "OS");
    let h_env = format!("{:<8}", "Env");
    let h_fmt = format!("{:<8}", "Format");
    let h_def = "Default";

    if colored {
        lines.push(format!(
            "{} {} {} {} {} {}",
            h_triple.bold(),
            h_arch.bold(),
            h_os.bold(),
            h_env.bold(),
            h_fmt.bold(),
            h_def.bold()
        ));
    } else {
        lines.push(format!("{h_triple} {h_arch} {h_os} {h_env} {h_fmt} {h_def}"));
    }

    for target in all {
        lines.push(format_target_row(*target, host, colored));
    }

    lines.join("\n")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_targets_output_plain_contains_all_targets() {
        let out = targets_output(false);
        assert!(out.contains(&format!(
            "Yukino Supported Target Triples ({} available)",
            TargetTriple::all().len()
        )));
        assert!(out.contains("Target Triple"));
        assert!(out.contains("x86_64-unknown-linux-gnu"));
        assert!(out.contains("x86_64-pc-windows-gnu"));
        assert!(out.contains("x86_64-pc-windows-msvc"));
        assert!(out.contains("aarch64-linux-android"));
        assert!(out.contains("[host]"));
    }

    #[test]
    fn test_targets_output_colored_contains_ansi() {
        let out = targets_output(true);
        assert!(out.contains("\x1b["));
        assert!(out.contains("[host]"));
    }
}

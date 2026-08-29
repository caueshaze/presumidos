//! Comandos operacionais do processo.
use crate::{auth, config, custom_event_manifest, db, operability, startup};
#[derive(Debug)]
struct BootstrapAdminArgs {
    username: String,
    email: String,
    password: String,
}

fn parse_bootstrap_admin_args<I>(mut args: I) -> Result<BootstrapAdminArgs, String>
where
    I: Iterator<Item = String>,
{
    let mut username = None;
    let mut email = None;

    while let Some(flag) = args.next() {
        match flag.as_str() {
            "--username" => username = args.next(),
            "--email" => email = args.next(),
            unknown => {
                return Err(format!(
                    "argumento desconhecido: {unknown}. Use --username e --email."
                ));
            }
        }
    }

    let password = if let Ok(value) = std::env::var("BOOTSTRAP_ADMIN_PASSWORD") {
        value
    } else {
        let first =
            rpassword::prompt_password("Senha do admin inicial: ").map_err(|e| e.to_string())?;
        let second = rpassword::prompt_password("Confirme a senha: ").map_err(|e| e.to_string())?;
        if first != second {
            return Err("as senhas digitadas nao conferem".to_string());
        }
        first
    };

    Ok(BootstrapAdminArgs {
        username: username
            .ok_or_else(|| "faltou --username para o bootstrap inicial".to_string())?,
        email: email.ok_or_else(|| "faltou --email para o bootstrap inicial".to_string())?,
        password,
    })
}

fn run_import_custom_event_command<I>(mut args: I) -> i32
where
    I: Iterator<Item = String>,
{
    let mut file = None;
    let mut apply = false;
    while let Some(arg) = args.next() {
        match arg.as_str() {
            "--file" => file = args.next(),
            "--apply" => apply = true,
            "--dry-run" => apply = false,
            _ => {
                eprintln!("uso: import-custom-event --file <arquivo> [--dry-run|--apply]");
                return 2;
            }
        }
    }
    let Some(file) = file else {
        eprintln!("--file é obrigatório");
        return 2;
    };
    let bytes = match std::fs::read_to_string(&file) {
        Ok(v) => v,
        Err(e) => {
            eprintln!("falha ao ler manifesto: {e}");
            return 1;
        }
    };
    let manifest = match custom_event_manifest::parse_and_validate(&bytes) {
        Ok(v) => v,
        Err(e) => {
            eprintln!("manifesto inválido: {e}");
            return 2;
        }
    };
    let rt = tokio::runtime::Runtime::new().unwrap();
    let result = rt.block_on(async {
        db::init().await;
        custom_event_manifest::import(manifest, apply).await
    });
    match result {
        Ok((items, options)) => {
            println!(
                "{}: {items} itens, {options} opções",
                if apply { "importado" } else { "dry-run" }
            );
            0
        }
        Err(e) => {
            eprintln!("falha na importação: {e}");
            1
        }
    }
}

fn run_cleanup_expired_command() -> i32 {
    let runtime = tokio::runtime::Runtime::new().expect("falha ao criar runtime tokio");
    let result = runtime.block_on(async {
        db::init().await;
        startup::run_housekeeping().await
    });

    match result {
        Ok(()) => 0,
        Err(error) => {
            eprintln!("falha no cleanup-expired: {error}");
            1
        }
    }
}

fn run_check_config_command() -> i32 {
    match config::check_config() {
        Ok(()) => {
            println!("configuração válida");
            0
        }
        Err(error) => {
            eprintln!("configuração inválida: {error}");
            78
        }
    }
}

fn run_migrate_command<I>(mut args: I) -> i32
where
    I: Iterator<Item = String>,
{
    let mut check_only = false;
    while let Some(arg) = args.next() {
        match arg.as_str() {
            "--check" => check_only = true,
            _ => {
                eprintln!("uso: migrate [--check]");
                return 2;
            }
        }
    }
    if let Err(error) = config::check_config() {
        eprintln!("configuração inválida: {error}");
        return 78;
    }
    let runtime = tokio::runtime::Runtime::new().expect("falha ao criar runtime tokio");
    if check_only {
        return match runtime.block_on(db::migration_report()) {
            Ok(report) if !report.pending && !report.dirty && !report.checksum_mismatch => {
                println!("migrations em dia: {}/{}", report.applied, report.expected);
                0
            }
            Ok(report) => {
                eprintln!(
                    "migrations incompatíveis: aplicadas={}, esperadas={}, pendentes={}, dirty={}, checksum_mismatch={}",
                    report.applied, report.expected, report.pending, report.dirty, report.checksum_mismatch
                );
                1
            }
            Err(error) => {
                eprintln!("falha ao verificar migrations: {error}");
                1
            }
        };
    }
    match runtime.block_on(db::apply_migrations()) {
        Ok(()) => {
            println!("migrations aplicadas");
            0
        }
        Err(error) => {
            eprintln!("falha ao aplicar migrations: {error}");
            1
        }
    }
}

fn run_db_command<I>(mut args: I) -> i32
where
    I: Iterator<Item = String>,
{
    match args.next().as_deref() {
        Some("check") if args.next().is_none() => {}
        _ => {
            eprintln!("uso: db check");
            return 2;
        }
    }
    if let Err(error) = config::check_config() {
        eprintln!("configuração inválida: {error}");
        return 78;
    }
    let runtime = tokio::runtime::Runtime::new().expect("falha ao criar runtime tokio");
    match runtime.block_on(db::integrity_check_without_migration()) {
        Ok(result) if result == "ok" => {
            println!("integrity_check: ok");
            0
        }
        Ok(result) => {
            eprintln!("integrity_check: {result}");
            1
        }
        Err(error) => {
            eprintln!("falha no db check: {error}");
            1
        }
    }
}

#[path = "cli/backup.rs"]
mod backup;
use backup::run_backup_command;

pub(crate) fn try_handle_server_command() -> Option<i32> {
    let mut args = std::env::args().skip(1);
    let command = args.next()?;
    if command == "import-custom-event" {
        return Some(run_import_custom_event_command(args));
    }
    if command == "cleanup-expired" {
        return Some(run_cleanup_expired_command());
    }
    if command == "check-config" {
        return Some(run_check_config_command());
    }
    if command == "migrate" {
        return Some(run_migrate_command(args));
    }
    if command == "db" {
        return Some(run_db_command(args));
    }
    if command == "backup" {
        return Some(run_backup_command(args));
    }
    if command != "bootstrap-admin" {
        return None;
    }

    let parsed = match parse_bootstrap_admin_args(args) {
        Ok(parsed) => parsed,
        Err(error) => {
            eprintln!("{error}");
            eprintln!(
                "uso: cargo run -p ferrugem-web --features server -- bootstrap-admin --username <usuario> --email <email>"
            );
            return Some(2);
        }
    };

    let runtime = tokio::runtime::Runtime::new().expect("falha ao criar runtime tokio");
    let result = runtime.block_on(async {
        db::init().await;
        auth::run_bootstrap_admin(
            parsed.username,
            parsed.email,
            parsed.password,
            crate::config::settings().admin_bootstrap_secret.clone(),
        )
        .await
    });

    match result {
        Ok(user) => {
            println!(
                "admin inicial criado com sucesso: {} <{}>",
                user.username, user.email
            );
            Some(0)
        }
        Err(error) => {
            eprintln!("falha no bootstrap do admin inicial: {error}");
            Some(1)
        }
    }
}

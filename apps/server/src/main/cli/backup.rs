use crate::{config, db, operability};

pub(super) fn run_backup_command<I>(mut args: I) -> i32
where
    I: Iterator<Item = String>,
{
    let Some(action) = args.next() else {
        eprintln!("uso: backup create --output <diretório> | backup verify <diretório> | backup restore ...");
        return 2;
    };
    match action.as_str() {
        "create" => {
            let mut output = None;
            while let Some(arg) = args.next() {
                if arg == "--output" {
                    output = args.next();
                } else {
                    eprintln!("uso: backup create --output <diretório>");
                    return 2;
                }
            }
            let Some(output) = output else {
                eprintln!("--output é obrigatório");
                return 2;
            };
            if let Err(error) = config::check_config() {
                eprintln!("configuração inválida: {error}");
                return 78;
            }
            let runtime = tokio::runtime::Runtime::new().expect("falha ao criar runtime tokio");
            match runtime.block_on(async {
                db::init_for_backup().await;
                operability::create_backup(std::path::Path::new(&output)).await
            }) {
                Ok(path) => {
                    println!("backup criado: {}", path.display());
                    0
                }
                Err(error) => {
                    eprintln!("backup falhou: {error}");
                    1
                }
            }
        }
        "verify" => {
            let Some(path) = args.next() else {
                eprintln!("uso: backup verify <diretório>");
                return 2;
            };
            if args.next().is_some() {
                eprintln!("uso: backup verify <diretório>");
                return 2;
            }
            let runtime = tokio::runtime::Runtime::new().expect("falha ao criar runtime tokio");
            match runtime.block_on(operability::verify_backup(std::path::Path::new(&path))) {
                Ok(()) => {
                    println!("backup válido");
                    0
                }
                Err(error) => {
                    eprintln!("backup inválido: {error}");
                    1
                }
            }
        }
        "restore" => {
            let mut input = None;
            let mut database = None;
            let mut assets = None;
            let mut replace = false;
            while let Some(arg) = args.next() {
                match arg.as_str() {
                    "--input" => input = args.next(),
                    "--database" => database = args.next(),
                    "--assets" => assets = args.next(),
                    "--replace" => replace = true,
                    _ => {
                        eprintln!("uso: backup restore --input <dir> --database <path> --assets <dir> [--replace]");
                        return 2;
                    }
                }
            }
            let (Some(input), Some(database), Some(assets)) = (input, database, assets) else {
                eprintln!("--input, --database e --assets são obrigatórios");
                return 2;
            };
            let runtime = tokio::runtime::Runtime::new().expect("falha ao criar runtime tokio");
            let result = runtime.block_on(operability::verify_backup(std::path::Path::new(&input)));
            if let Err(error) = result {
                eprintln!("restore recusado: {error}");
                return 1;
            }
            match operability::restore_backup(
                std::path::Path::new(&input),
                std::path::Path::new(&database),
                std::path::Path::new(&assets),
                replace,
            ) {
                Ok(()) => {
                    println!("restore concluído; inicialize a aplicação e valide readiness");
                    0
                }
                Err(error) => {
                    eprintln!("restore falhou: {error}");
                    1
                }
            }
        }
        _ => {
            eprintln!("ação de backup desconhecida");
            2
        }
    }
}

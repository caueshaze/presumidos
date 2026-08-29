fn sha256_file(path: &Path) -> Result<String, String> {
    let mut file =
        File::open(path).map_err(|e| format!("não foi possível ler {}: {e}", path.display()))?;
    let mut hasher = Sha256::new();
    let mut buffer = [0u8; 64 * 1024];
    loop {
        let read = file
            .read(&mut buffer)
            .map_err(|e| format!("falha ao calcular checksum: {e}"))?;
        if read == 0 {
            break;
        }
        hasher.update(&buffer[..read]);
    }
    Ok(hex::encode(hasher.finalize()))
}

fn collect_files(
    root: &Path,
    current: &Path,
    output: &mut Vec<(PathBuf, PathBuf)>,
) -> Result<(), String> {
    let entries = fs::read_dir(current).map_err(|e| format!("falha ao listar assets: {e}"))?;
    for entry in entries {
        let entry = entry.map_err(|e| format!("falha ao listar assets: {e}"))?;
        let path = entry.path();
        if path.file_name().is_some_and(|name| name == ".staging") {
            // staging incompleto não faz parte do estado publicado do AssetStore.
            continue;
        }
        if path.is_dir() {
            collect_files(root, &path, output)?;
        } else if path.is_file() {
            let relative = path
                .strip_prefix(root)
                .map_err(|_| "asset fora da raiz configurada".to_string())?
                .to_path_buf();
            output.push((path, relative));
        }
    }
    Ok(())
}

fn archive_assets(root: &Path, output: &Path) -> Result<(), String> {
    let file =
        File::create(output).map_err(|e| format!("falha ao criar archive de assets: {e}"))?;
    let mut writer = ZipWriter::new(file);
    let options = FileOptions::default().compression_method(zip::CompressionMethod::Deflated);
    if root.exists() {
        let mut entries = Vec::new();
        collect_files(root, root, &mut entries)?;
        entries.sort_by(|left, right| left.1.cmp(&right.1));
        for (source, relative) in entries {
            let name = relative.to_string_lossy().replace('\\', "/");
            writer
                .start_file(name, options)
                .map_err(|e| format!("falha ao criar archive de assets: {e}"))?;
            let mut input = File::open(source).map_err(|e| format!("falha ao ler asset: {e}"))?;
            std::io::copy(&mut input, &mut writer)
                .map_err(|e| format!("falha ao compactar asset: {e}"))?;
        }
    }
    writer
        .finish()
        .map_err(|e| format!("falha ao finalizar archive de assets: {e}"))?;
    Ok(())
}

async fn count_rows(pool: &SqlitePool, table: &str) -> Result<i64, sqlx::Error> {
    let query = format!("SELECT COUNT(*) FROM {table}");
    let row: (i64,) = sqlx::query_as(&query).fetch_one(pool).await?;
    Ok(row.0)
}

async fn table_exists(pool: &SqlitePool, table: &str) -> Result<bool, String> {
    let row: (i64,) =
        sqlx::query_as("SELECT COUNT(*) FROM sqlite_master WHERE type = 'table' AND name = ?1")
            .bind(table)
            .fetch_one(pool)
            .await
            .map_err(|error| format!("falha ao verificar tabela {table}: {error}"))?;
    Ok(row.0 != 0)
}

async fn count_rows_if_present(pool: &SqlitePool, table: &str) -> Result<i64, String> {
    if table_exists(pool, table).await? {
        count_rows(pool, table)
            .await
            .map_err(|error| format!("falha ao contar {table}: {error}"))
    } else {
        Ok(0)
    }
}

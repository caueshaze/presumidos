use super::*;

#[tokio::test]
#[ignore = "smoke explícito usa processos e dois bancos SQLite independentes"]
async fn package_promotion_two_sqlite_smoke() {
    if let Ok(stage) = std::env::var("PACKAGE_SMOKE_STAGE") {
        run_package_smoke_stage(&stage).await;
        return;
    }
    let root =
        std::env::temp_dir().join(format!("presumidos-package-smoke-{}", uuid::Uuid::new_v4()));
    fs::create_dir_all(&root).expect("diretório do smoke");
    let executable = std::env::current_exe().expect("binário de testes");
    for stage in [
        "dev-a",
        "prod-a",
        "dev-b",
        "prod-b",
        "dev-c",
        "prod-c",
        "vma-dev-a",
        "vma-prod-a",
        "vma-dev-b",
        "vma-prod-b",
    ] {
        let status = Command::new(&executable)
            .arg("--exact")
            .arg("http_tests::package_promotion_two_sqlite_smoke")
            .arg("--ignored")
            .arg("--nocapture")
            .env("PACKAGE_SMOKE_ROOT", &root)
            .env("PACKAGE_SMOKE_STAGE", stage)
            .status()
            .expect("iniciar etapa do smoke");
        assert!(status.success(), "etapa {stage} falhou");
    }
    let _ = fs::remove_dir_all(root);
}

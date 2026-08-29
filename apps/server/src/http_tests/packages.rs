use super::*;

#[path = "packages/support.rs"]
mod support;
use support::*;
#[path = "packages/admin_http.rs"]
mod admin_http;
#[path = "packages/builder_http.rs"]
mod builder_http;
#[path = "packages/builder_setup.rs"]
mod builder_setup;
use builder_setup::*;
#[path = "packages/dev_a.rs"]
mod dev_a;
#[path = "packages/dev_b.rs"]
mod dev_b;
#[path = "packages/dev_c.rs"]
mod dev_c;
#[path = "packages/prod_a.rs"]
mod prod_a;
#[path = "packages/prod_b.rs"]
mod prod_b;
#[path = "packages/prod_c.rs"]
mod prod_c;
#[path = "packages/promotion_smoke.rs"]
mod promotion_smoke;
#[path = "packages/vma_dev_a.rs"]
mod vma_dev_a;
#[path = "packages/vma_dev_b.rs"]
mod vma_dev_b;
#[path = "packages/vma_prod_a.rs"]
mod vma_prod_a;
#[path = "packages/vma_prod_b.rs"]
mod vma_prod_b;

pub(super) async fn run_package_smoke_stage(stage: &str) {
    let root = package_smoke_root();
    let dev_db = root.join("dev.db");
    let prod_db = root.join("prod.db");
    let dev_assets = root.join("dev-assets");
    let prod_assets = root.join("prod-assets");
    let dev_slug = "package-promotion-smoke";
    match stage {
        "dev-a" => {
            dev_a::run(
                &root,
                &dev_db,
                &prod_db,
                &dev_assets,
                &prod_assets,
                dev_slug,
            )
            .await
        }
        "prod-a" => {
            prod_a::run(
                &root,
                &dev_db,
                &prod_db,
                &dev_assets,
                &prod_assets,
                dev_slug,
            )
            .await
        }
        "dev-b" => {
            dev_b::run(
                &root,
                &dev_db,
                &prod_db,
                &dev_assets,
                &prod_assets,
                dev_slug,
            )
            .await
        }
        "prod-b" => {
            prod_b::run(
                &root,
                &dev_db,
                &prod_db,
                &dev_assets,
                &prod_assets,
                dev_slug,
            )
            .await
        }
        "dev-c" => {
            dev_c::run(
                &root,
                &dev_db,
                &prod_db,
                &dev_assets,
                &prod_assets,
                dev_slug,
            )
            .await
        }
        "prod-c" => {
            prod_c::run(
                &root,
                &dev_db,
                &prod_db,
                &dev_assets,
                &prod_assets,
                dev_slug,
            )
            .await
        }
        "vma-dev-a" => {
            vma_dev_a::run(
                &root,
                &dev_db,
                &prod_db,
                &dev_assets,
                &prod_assets,
                dev_slug,
            )
            .await
        }
        "vma-prod-a" => {
            vma_prod_a::run(
                &root,
                &dev_db,
                &prod_db,
                &dev_assets,
                &prod_assets,
                dev_slug,
            )
            .await
        }
        "vma-dev-b" => {
            vma_dev_b::run(
                &root,
                &dev_db,
                &prod_db,
                &dev_assets,
                &prod_assets,
                dev_slug,
            )
            .await
        }
        "vma-prod-b" => {
            vma_prod_b::run(
                &root,
                &dev_db,
                &prod_db,
                &dev_assets,
                &prod_assets,
                dev_slug,
            )
            .await
        }
        _ => panic!("stage de package smoke desconhecido: {stage}"),
    }
}

mod config;
mod manager;
mod pass;

use anyhow::Result;
use config::EvaluationContext;
use database::storage::Context;
use manager::EvaluationManager;
use tracing_subscriber::layer::SubscriberExt;
use tracing_subscriber::util::SubscriberInitExt;
use tracing_subscriber::EnvFilter;

fn init_logger() {
    tracing_subscriber::registry()
        .with(EnvFilter::try_from_default_env().unwrap_or_else(|_| EnvFilter::new("info")))
        .with(
            tracing_subscriber::fmt::layer()
                .with_file(true)
                .with_line_number(true)
                .with_thread_ids(true)
                .with_thread_names(true)
                .with_target(false),
        )
        .init();
}

#[tokio::main]
async fn main() -> Result<()> {
    // 初始化日志记录器
    init_logger();

    // 加载配置
    let ctx = EvaluationContext::load_config("evaluate/config")?;

    // 初始化数据库连接
    let db_ctx = Context::new(&ctx.database_url.url, "".into()).await;

    // 初始化 EvaluationManager 并添加默认 Pass
    let mut manager = EvaluationManager::new();
    manager.add_default_passes();

    // 运行评估并获取总分数
    manager.run(&ctx, &db_ctx).await?;

    Ok(())
}

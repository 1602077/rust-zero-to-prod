use z2p::{configuration::get_config, startup::Application, telemetry};

#[tokio::main]
async fn main() -> std::io::Result<()> {
    let subscriber = telemetry::get_subscriber(
        "zero2prod".into(),
        "info".into(),
        std::io::stdout,
    );

    telemetry::init_subscriber(subscriber);

    let config = get_config().expect("failed to read config");

    Application::build(config)
        .await?
        .run_until_stopped()
        .await?;

    Ok(())
}

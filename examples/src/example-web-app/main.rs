use arena_examples::example_axum_web_server::ExampleAxumWebApp;

#[tokio::main]
async fn main() {
    env_logger::init();

    let args: Vec<String> = std::env::args().collect();

    if args.len() != 7 {
        eprintln!(
            "Usage: {} <web_app_port> <postgres_connection_string> <kafka_bootstrap> <calibration_url> <mssql_connection_string> <oauth_issuer_url>\n\
             Environment: OAUTH_TLS_CA_PEM (required) — PEM of the OAuth AS TLS certificate (or CA) for HTTPS calls to the issuer.",
            args[0]
        );
        std::process::exit(1);
    }

    let oauth_tls_ca_pem = std::env::var("OAUTH_TLS_CA_PEM").unwrap_or_else(|_| {
        eprintln!("Missing required environment variable OAUTH_TLS_CA_PEM");
        std::process::exit(1);
    });

    let web_app_port: u16 = args[1]
        .parse()
        .expect("web_app_port must be a valid port number");
    let postgres_connection_string = &args[2];
    let kafka_bootstrap = &args[3];
    let calibration_url = &args[4];
    let mssql_connection_string = &args[5];
    let oauth_issuer_url = &args[6];
    let kafka_topic = "readings";

    let web_app = ExampleAxumWebApp::new(
        postgres_connection_string,
        kafka_bootstrap,
        kafka_topic,
        calibration_url,
        mssql_connection_string,
        oauth_issuer_url,
        &oauth_tls_ca_pem,
    )
    .await;

    let (_tx, rx) = tokio::sync::oneshot::channel();

    tokio::select! {
        result = web_app.serve(web_app_port, rx) => {
            if let Err(e) = result {
                log::error!("web app error: {}", e);
            }
        }
        _ = tokio::signal::ctrl_c() => {
            log::info!("shutting down");
        }
    }
}

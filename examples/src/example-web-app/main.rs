use crate::example_axum_web_server::ExampleAxumWebApp;

#[path = "api/readings-api.rs"]
mod example_axum_web_server;

#[tokio::main]
async fn main() {
    env_logger::init();

    let args: Vec<String> = std::env::args().collect();
    
    if args.len() != 4 {
        eprintln!("Usage: {} <web_app_port> <postgres_connection_string> <kafka_bootstrap>", args[0]);
        std::process::exit(1);
    }

    let web_app_port: u16 = args[1].parse().expect("web_app_port must be a valid port number");
    let postgres_connection_string = &args[2];
    let kafka_bootstrap = &args[3];
    let kafka_topic = "readings";

    let web_app = ExampleAxumWebApp::new(postgres_connection_string, kafka_bootstrap, kafka_topic).await;
    
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

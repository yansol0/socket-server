use std::{
    collections::HashSet,
    env,
    io::Error as IoError,
    path::Path, sync::{Arc, Mutex},
};
use std::collections::HashMap;
use tokio::net::TcpListener;
use tokio::sync::mpsc;
use socket_server::peers::peers::PeerMap;
use socket_server::queue::consumer;
use socket_server::socket;

type ChannelMap = Arc<Mutex<HashMap<String, HashSet<String>>>>;

#[tokio::main]
async fn main() -> Result<(), IoError> {
    match dotenvy::from_path(Path::new(".env")) {
        Ok(()) => println!(".env read successfully"),
        Err(e) => panic!("Could not load .env file: {}", e),
    };

    let broker_uri = env::var("RABBITMQ_URI")
        .expect("Missing RabbitMQ URI");

    let addr = env::args().nth(1)
        .unwrap_or_else(|| "127.0.0.1:3000".to_string());

    let user_state = PeerMap::new(Mutex::new(Vec::new()));

    let channel_state = ChannelMap::new(Mutex::new(HashMap::new()));
    let try_socket = TcpListener::bind(&addr).await;

    let listener = try_socket
        .expect("Failed to bind socket");

    let broker_uri_clone = broker_uri.clone();

    let (sender, receiver) = mpsc::channel::<consumer::BrokerMessage>(100);
    let receiver = Arc::new(Mutex::new(receiver));

    let rabbitmq_handle = tokio::spawn(async move {
        match consumer::subscribe_to_broker(&broker_uri_clone, sender).await {
            Ok(()) => println!("Connected to broker"),
            Err(e) => println!("Failed to connect to broker: {:?}", e)
        }
    });

    let server_handle = tokio::spawn(async move {
        while let Ok((stream, addr)) = listener.accept().await {
            let rx = receiver.clone();
            tokio::spawn(socket::handle_connection(user_state.clone(), channel_state.clone(), stream, addr, rx));
        }
    });

    tokio::select! {
        _ = rabbitmq_handle => println!("RabbitMQ handler completed"),
        _ = server_handle => println!("Server handler completed"),
    }

    return Ok(());
}


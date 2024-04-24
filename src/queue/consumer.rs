use std::env;
use amiquip::{Connection , ConsumerMessage, ConsumerOptions, QueueDeclareOptions, Result};
use serde::{Serialize, Deserialize};
use tokio_tungstenite::tungstenite::Message;
use tokio::sync::mpsc::Sender;
use crate::auth::auth;

#[derive(Serialize, Deserialize)]
enum MessageContentType {
    Message,
    Channel
}

#[derive(Serialize, Deserialize)]
pub struct MessageContent {
    id: String,
    channel_id: String,
    sent_at: String,
    sent_by: String,
    content: String,
    reply_to: Option<String>,
}

#[derive(Serialize, Deserialize)]
pub struct BrokerMessage {
    token: String,
    pub user_id: String,
    pub channel: String,
    pub content: MessageContent,
}

impl BrokerMessage {
    pub fn from_body(body: &str) -> Result<Self, serde_json::Error> {
        return serde_json::from_str(body);
    }
}

impl From<BrokerMessage> for Message {
    fn from(message: BrokerMessage) -> Self {
        let content_json = serde_json::to_string(&message.content)
            .expect("Failed to serialize content to JSON");

        return Message::Text(content_json);
    }
}

pub async fn subscribe_to_broker(conn_str: &str, sender: Sender<BrokerMessage> ) -> Result<()> {
    let mut connection = Connection::insecure_open(conn_str).unwrap();
    let channel = connection.open_channel(None).unwrap();
    let queue = channel.queue_declare("chat", QueueDeclareOptions::default()).unwrap();
    let consumer = queue.consume(ConsumerOptions::default()).unwrap();
    let jwt_secret = env::var("JWT_SECRET").unwrap();

    for message in consumer.receiver().iter() {
        match message {
            ConsumerMessage::Delivery(delivery) => {
                let body = String::from_utf8_lossy(&delivery.body);
                let broker_message = BrokerMessage::from_body(&body).unwrap();

                match auth::verify_token(&broker_message.token, &jwt_secret) {
                    Ok(_) => println!("Valid token"),
                    Err(e) => println!("Unable to authenticate: {:?}", e)
                }

                sender.send(broker_message).await?;
                consumer.ack(delivery)?;
            }
            other => {
                println!("Consumer ended: {:?}", other);
                break;
            }
        }
    }

    return connection.close();
}


use std::sync::{Arc, Mutex};
use std::env;
use std::collections::{HashSet, HashMap};
use std::net::SocketAddr;
use tokio_tungstenite::tungstenite::protocol::Message;
use futures_channel::mpsc::{unbounded, UnboundedSender};
use futures_util::{future, stream::TryStreamExt, StreamExt};
use tokio::net::TcpStream;
use serde::{Serialize, Deserialize};
use tokio::sync::mpsc::Receiver;
use anyhow::Result;

use crate::auth::auth;
use crate::data::{db, redis_handler};
use crate::peers::peers::{Peer, PeerMap, PeerMethods};
use crate::queue::consumer;

type ChannelMap = Arc<Mutex<HashMap<String, HashSet<String>>>>;

#[derive(Serialize, Deserialize)]
enum ClientEventType {
    Typing(TypingEvent),
    ChannelRead
}

#[derive(Serialize, Deserialize)]
struct TypingEvent {
    is_typing: bool,
}

#[derive(Serialize, Deserialize)]
pub struct ClientEvent {
    token: String,
    channel: String,
    event_type: ClientEventType,
    user_id: String
}

#[derive(Serialize, Deserialize)]
struct AuthMessage {
    token: String,
}

impl From<AuthMessage> for Message {
    fn from(auth_message: AuthMessage) -> Message {
        let content_json = serde_json::to_string(&auth_message.token).unwrap();
        return Message::Text(content_json);
    }
}

pub async fn handle_connection(
    peer_map: PeerMap,
    channel_map: ChannelMap,
    raw_stream: TcpStream,
    addr: SocketAddr,
    _receiver: Arc<Mutex<Receiver<consumer::BrokerMessage>>>
) -> Result<()> {
    let ws_stream = tokio_tungstenite::accept_async(raw_stream).await?;
    let (tx, rx) = unbounded();
    let (outgoing, mut incoming) = ws_stream.split();

    while let Some(Ok(msg)) = incoming.next().await {
        let socket_message = msg.to_text().unwrap();

        let existing_peer = peer_map.lock().unwrap()
            .iter()
            .any(|peer| peer.addr == addr.clone());

        if !existing_peer {
            join_peer(socket_message, &peer_map, &channel_map, addr, &tx).await?
        }

        //  If message isnt auth message and user is authenticated, handle message
    }

    let broadcast_incoming = incoming.try_for_each(|msg| {
        println!("Handle message here: {:?}", msg);
        return future::ok(());
    });

    let receive_from_others = rx.map(Ok).forward(outgoing);

    future::select(broadcast_incoming, receive_from_others).await;

    disconnect_peer(&peer_map, &channel_map, addr).await?;

    return Ok(());
}

async fn transmit_broker_messages(
    receiver: Arc<Mutex<Receiver<consumer::BrokerMessage>>>,
    peer_map: PeerMap,
    addr: SocketAddr,
) -> Result<()> {
    loop {
        // I think locking this "indefinitely" is ok seeing as this is the only place receiver is accessed
        match receiver.lock().unwrap().try_recv() {
            Ok(broker_msg) => {
                let channel = &broker_msg.channel;
                let channel_users = redis_handler::get_channel_users(channel).await?;
                let message: Message = broker_msg.into();
                let peers = peer_map.lock().unwrap();

                let broadcast_recipients: Vec<_> = peers
                    .iter()
                    .filter(|peer| channel_users.contains(&peer.user_id) && peer.addr != addr)
                    .map(|peer| &peer.tx)
                    .collect();

                for recp in broadcast_recipients {
                    recp.unbounded_send(message.clone()).unwrap();
                }
            },
            Err(err) => {
                println!("RabbitMQ message stream closed: {}", err);
                break;
            }
        }
    }

    return Ok(());
}

async fn disconnect_peer(peer_map: &PeerMap, channel_map: &ChannelMap, addr: SocketAddr) -> Result<()> {
    let peer_user_id = PeerMethods::find_peer_id(peer_map, addr);
    let user_channels = db::fetch_channels(&peer_user_id).await?;

    redis_handler::remove_user(&peer_user_id, user_channels).await?;

    //TODO: Remove user from channel_map

    PeerMethods::remove_peer(peer_map, addr);

    return Ok(());
}

async fn join_peer(
    msg: &str,
    peer_map: &PeerMap,
    channel_map: &ChannelMap,
    addr: SocketAddr,
    tx: &UnboundedSender<Message>
) -> Result<()> {
    let jwt_secret = env::var("JWT_SECRET")
        .expect("Missing secret");
    let body: AuthMessage = serde_json::from_str(msg)?;
    let token = auth::verify_token(&body.token, &jwt_secret)?;
    let user_channels = db::fetch_channels(&token.claims.id).await?;

    peer_map.lock().unwrap().push(Peer::new(token.claims.id.clone(), addr, tx.clone()));

    for channel in user_channels.iter() {
        let mut locked_channels = channel_map.lock().unwrap();
        let user_id = token.claims.id.clone();

        if locked_channels.contains_key(channel) {
            locked_channels.entry(channel.to_string())
                .and_modify(|v| { v.insert(user_id.clone()); } )
                .or_insert(HashSet::from([user_id.clone()]));
        } else {
            let mut new_channel_users = HashSet::new();
            new_channel_users.insert(user_id);
            locked_channels.insert(channel.clone(), new_channel_users);
        }
    }

    return Ok(redis_handler::add_user_to_channels(&token.claims.id, user_channels).await?);
}

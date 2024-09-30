use actix::{Actor, ActorContext, Addr, AsyncContext, StreamHandler};
use actix_web_actors::ws;
use serde::Serialize;
use std::collections::{HashMap, HashSet};
use std::sync::{Arc, Mutex};
use uuid::Uuid;

pub struct ChatRoom {
    sessions: Arc<Mutex<HashSet<Addr<MyWebSocket>>>>,
    user_sessions: Arc<Mutex<HashMap<Uuid, Addr<MyWebSocket>>>>, // Store user sessions
}

#[derive(Serialize)]
struct MessageDTO {
    message_type: String,
}

impl ChatRoom {
    pub fn new() -> Self {
        ChatRoom {
            sessions: Arc::new(Mutex::new(HashSet::new())),
            user_sessions: Arc::new(Mutex::new(HashMap::new())),
        }
    }

    pub fn send_message(&self, user_ids: &Vec<Uuid>, message: &str) {
        let user_sessions = self.user_sessions.lock().unwrap();
        for user_id in user_ids {
            if let Some(session) = user_sessions.get(user_id) {
                session.do_send(MyMessage(message.to_string()));
            }
        }
    }

    pub fn add_session(&self, user_id: Uuid, addr: Addr<MyWebSocket>) {
        self.sessions.lock().unwrap().insert(addr.clone());
        self.user_sessions.lock().unwrap().insert(user_id, addr);

        let user_ids: Vec<Uuid> = self.user_sessions.lock().unwrap().keys().cloned().collect();
        let update_message = MessageDTO {
            message_type: "UPDATE_USERS".to_string(),
        };

        self.send_message(&user_ids, &serde_json::to_string(&update_message).unwrap());
    }

    pub fn remove_session(&self, user_id: Uuid, addr: &Addr<MyWebSocket>) {
        self.sessions.lock().unwrap().remove(addr);
        self.user_sessions.lock().unwrap().remove(&user_id);

        let user_ids: Vec<Uuid> = self.user_sessions.lock().unwrap().keys().cloned().collect();
        let update_message = MessageDTO {
            message_type: "UPDATE_USERS".to_string(),
        };

        self.send_message(&user_ids, &serde_json::to_string(&update_message).unwrap());
    }

    pub fn get_connected_user_ids(&self) -> Vec<Uuid> {
        self.user_sessions.lock().unwrap().keys().cloned().collect()
    }
}

// Define a custom message type
#[derive(actix::Message)]
#[rtype(result = "()")]
pub struct MyMessage(pub String);

// WebSocket connection actor
pub struct MyWebSocket {
    pub room: Arc<ChatRoom>,
    pub user_id: Uuid,
}

impl Actor for MyWebSocket {
    type Context = ws::WebsocketContext<Self>;

    fn started(&mut self, ctx: &mut Self::Context) {
        self.room.add_session(self.user_id, ctx.address());
    }

    fn stopped(&mut self, ctx: &mut Self::Context) {
        self.room.remove_session(self.user_id, &ctx.address());
    }
}

// Message handler for custom message type
impl actix::Handler<MyMessage> for MyWebSocket {
    type Result = ();

    fn handle(&mut self, msg: MyMessage, ctx: &mut Self::Context) {
        ctx.text(msg.0);
    }
}

// Message handler for WebSocket messages
impl StreamHandler<Result<ws::Message, ws::ProtocolError>> for MyWebSocket {
    fn handle(&mut self, msg: Result<ws::Message, ws::ProtocolError>, ctx: &mut Self::Context) {
        match msg {
            Ok(ws::Message::Ping(msg)) => {
                ctx.pong(&msg);
            }
            Ok(ws::Message::Pong(_)) => {}
            Ok(ws::Message::Binary(_)) => {}
            Ok(ws::Message::Close(_)) => {
                ctx.stop();
            }
            _ => {}
        }
    }
}

use std::time::Duration;
use actix::{Actor, ActorContext, AsyncContext, Handler, Message, StreamHandler};
use actix_web::{Error, HttpRequest, HttpResponse, web};
use actix_web_actors::ws;
use serde_json::json;
use tokio::time::sleep;
use crate::async_cacher::SharedAsyncCacher;

pub struct WebSocket;

impl Actor for WebSocket {
    type Context = ws::WebsocketContext<Self>;

    fn started(&mut self, ctx: &mut Self::Context) {
        ctx.text("Connected");
        let ctx = ctx.address();

        let cacher = SharedAsyncCacher.clone();

        actix::spawn( async move {
            loop {
                if !cacher.is_empty().await {
                    if let Some((key, value)) = cacher.pop().await {
                        ctx.send( TaskerMessage(
                            json!({
                                key.display().to_string(): value
                            }).to_string()
                        )).await.unwrap();
                    }
                }
                sleep(Duration::from_millis(10)).await
            }
        });
    }

}

impl StreamHandler<Result<ws::Message, ws::ProtocolError>> for WebSocket {
    fn handle(&mut self, msg: Result<ws::Message, ws::ProtocolError>, ctx: &mut Self::Context) {
        if let Ok(ws::Message::Close(reason)) = msg {
            ctx.close(reason);
            ctx.stop();
        }
    }
}

impl Handler<TaskerMessage> for WebSocket {
    type Result = ();

    fn handle(&mut self, msg: TaskerMessage, ctx: &mut Self::Context) {
        ctx.text(msg.0);
    }
}

pub(crate) async fn ws_index(r: HttpRequest, stream: web::Payload) -> Result<HttpResponse, Error> {
    ws::start(WebSocket, &r, stream)
}

#[derive(Message)]
#[rtype(result = "()")]
pub struct TaskerMessage(pub(crate) String);
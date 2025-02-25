use event::Event;
use futures::future;
use futures::future::Loop;
use hyper::header::HeaderValue;
use hyper::rt::{Future, Stream};
use hyper::{Body, Client, Request};
use hyper_tls::HttpsConnector;
use serde_json;
use slack::command::handle_command;
use slack::event::{RtmRecv, RtmSend};
use status::StatusPing;
use std::sync::mpsc::Sender;
use std::thread;
use tungstenite::{connect, Message};
use url::Url;

pub fn connect_to_slack(
    token: &'static str,
    bot_id: &'static str,
    listen_channel: &'static str,
    tx: Sender<Event>,
    status_tx: Sender<StatusPing>,
) {
    tokio::spawn(future::loop_fn((), move |_| {
        let https = HttpsConnector::new(2).unwrap();
        let client = Client::builder().build::<_, Body>(https);

        let mut req = Request::new(Body::from(""));
        *req.uri_mut() = ("https://slack.com/api/rtm.connect?token=".to_owned() + token)
            .parse()
            .unwrap();

        req.headers_mut().insert(
            hyper::header::CONTENT_TYPE,
            HeaderValue::from_static("application/x-www-form-urlencoded"),
        );
        let tx2 = tx.clone();
        let status_tx2 = status_tx.clone();
        client
            .request(req)
            .and_then(|res| res.into_body().concat2())
            .map_err(|err| {
                println!("Err in connect_to_slack: {}", err);
            })
            .and_then(move |body| {
                let resp: serde_json::Value = serde_json::from_slice(&body)
                    .expect("could not deserialize Chunk in connect_to_slack");
                let ws_url = match &resp["url"] {
                    serde_json::Value::String(s) => s,
                    _ => "",
                };

                let (mut socket, _) =
                    connect(Url::parse(ws_url).unwrap()).expect("Cannot connect in rtm_handler");

                let bot_ping = format!("<@{}> ", bot_id);

                let mut id = 0;

                loop {
                    let msg = socket.read_message();

                    if msg.is_err() {
                        break;
                    }

                    match msg.unwrap() {
                        Message::Text(text) => match serde_json::from_str(&text) {
                            Ok(message) => match message {
                                RtmRecv::Message { text, channel, .. } => {
                                    status_tx2.send(StatusPing::SlackPingReceived).unwrap();
                                    if text.starts_with(&bot_ping) && channel.eq(listen_channel) {
                                        id += 1;
                                        let text_reply = match handle_command(
                                            text[bot_ping.len()..].to_owned(),
                                            tx2.clone(),
                                        ) {
                                            Ok(s) => s,
                                            Err(e) => Some(e.message),
                                        };
                                        match text_reply {
                                            Some(reply) => {
                                                socket
                                                    .write_message(Message::Text(
                                                        serde_json::to_string(&RtmSend {
                                                            id: id,
                                                            type_: "message".to_owned(),
                                                            channel: channel,
                                                            text: reply,
                                                        })
                                                        .unwrap(),
                                                    ))
                                                    .unwrap();
                                            }
                                            _ => {}
                                        }
                                    }
                                }
                            },
                            _ => {}
                        },
                        Message::Ping(_) => {
                            status_tx2.send(StatusPing::SlackPingReceived).unwrap();
                        }
                        _ => {}
                    }
                }

                println!("Reconnecting to Slack in 7 seconds...");
                thread::sleep(std::time::Duration::from_millis(7000));
                Ok(Loop::Continue(()))
            })
    }));
}

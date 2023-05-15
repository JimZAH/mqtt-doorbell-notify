use futures::{executor::block_on, stream::StreamExt};
use notify_rust::Notification;
use paho_mqtt as mqtt;
use serde_derive::{Deserialize, Serialize};
use std::{process, time::Duration};

struct Config {
    host: String,
    name: String,
    topic: String,
    trigger: String,
    qos: i32,
}

#[derive(Serialize, Deserialize)]
struct Doorbell {
    time: String,
    rfreceived: Rf,
}

#[derive(Serialize, Deserialize)]
struct Rf {
    sync: usize,
    low: usize,
    high: usize,
    data: String,
    rfkey: String,
}

fn main() {
    let config = Config {
        host: "".to_string(),
        name: "Door_receiver".to_string(),
        topic: "".to_string(),
        trigger: "".to_string(),
        qos: 1,
    };

    let create_opts = mqtt::CreateOptionsBuilder::new_v3()
        .server_uri(&config.host)
        .client_id(&config.name)
        .finalize();

    let mut client = mqtt::AsyncClient::new(create_opts).unwrap_or_else(|e| {
        println!("Error creating the client: {:?}", e);
        process::exit(1);
    });

    if let Err(err) = block_on(async {
        // Get message stream before connecting.
        let mut strm = client.get_stream(25);

        let lwt = mqtt::Message::new(&config.topic, "ALIVE", mqtt::QOS_1);

        let conn_opts = mqtt::ConnectOptionsBuilder::new_v3()
            .keep_alive_interval(Duration::from_secs(30))
            .clean_session(false)
            .will_message(lwt)
            .finalize();

        client.connect(conn_opts).await?;

        client.subscribe(&config.topic, config.qos).await?;

        while let Some(msg_opt) = strm.next().await {
            if let Some(msg) = msg_opt {
                dbg!("{}", msg.payload_str().to_lowercase());
                let mut msg = msg.payload_str().to_lowercase();
                msg.push('}');
                if let Ok(cmd) = serde_json::from_str::<Doorbell>(&msg) {
                    dbg!("Doorbell: {}", &cmd.rfreceived.data);
                    if cmd.rfreceived.data == config.trigger {
                        Notification::new()
                            .summary("Doorbell")
                            .body("Someone is at the door!")
                            .show()
                            .unwrap();
                    }
                }
            } else {
                println!("Lost connection. Attempting reconnect.");
                while let Err(err) = client.reconnect().await {
                    println!("Error reconnecting: {}", err);
                    async_std::task::sleep(Duration::from_millis(1000)).await;
                }
            }
        }

        Ok::<(), mqtt::Error>(())
    }) {
        eprintln!("{}", err);
    }
}

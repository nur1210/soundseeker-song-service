use std::error::Error;
use amqprs::callbacks::{DefaultChannelCallback, DefaultConnectionCallback};
use amqprs::channel::{BasicAckArguments, BasicCancelArguments, BasicConsumeArguments, BasicPublishArguments, Channel, QueueDeclareArguments};
use amqprs::connection::{Connection, OpenConnectionArguments};
use std::{io, time};
use std::env::var;
use std::time::Duration;
use amqprs::{BasicProperties, DELIVERY_MODE_PERSISTENT, FieldName};
use lazy_static::lazy_static;
use tokio::spawn;
use tokio::time::sleep;
use crate::db::pg::{Fingerprint, PostgresRepository};
use crate::db::Repository;

lazy_static! {
    static ref CONN_DETAILS: RabbitConnect = RabbitConnect {
        host: var("RABBITMQ_HOST").expect("RABBITMQ_HOST must be set"),
        port: var("RABBITMQ_PORT").expect("RABBITMQ_PORT must be set").parse().unwrap(),
        username: var("RABBITMQ_USERNAME").expect("RABBITMQ_USERNAME must be set"),
        password: var("RABBITMQ_PASSWORD").expect("RABBITMQ_PASSWORD must be set"),
        virtual_host: var("RABBITMQ_VIRTUAL_HOST").expect("RABBITMQ_VIRTUAL_HOST must be set"),
    };
}

struct RabbitConnect {
    host: String,
    port: u16,
    username: String,
    password: String,
    virtual_host: String,
}

pub async fn consume_rabbitmq(repository: &PostgresRepository) {
    loop {
        let t1 = spawn(async {
            check_for_new_fingerprints(&CONN_DETAILS).await.unwrap()
        });
        let (song_name, fingerprints) = match t1.await {
            Ok(song) => song,
            Err(e) => {
                println!("Error: {}", e);
                return;
            }
        };
        println!("New song: {}, with {} fingerprints", song_name, fingerprints.len());
        match repository.index(&song_name, fingerprints).await {
            Ok(_) => send(&CONN_DETAILS, "added-songs-queue", &song_name).await,
            Err(e) => println!("Error indexing song: {}, error: {}", &song_name, e)
        }

        sleep(time::Duration::from_secs(2)).await;
    }
}

async fn check_for_new_fingerprints(connection_details: &RabbitConnect) -> Result<(String, Vec<Fingerprint>), Box<dyn Error>> {
    let queue = "fingerprint-queue".to_string();

    let mut connection = connect_to_rabbitmq(&connection_details).await;
    let mut channel = channel_rabbitmq(&connection).await;

    let args = BasicConsumeArguments::new(&queue, format!("{} ?", queue).as_str());

    if !connection.is_open() {
        println!("Connection not open");
        connection = connect_to_rabbitmq(connection_details).await;
        channel = channel_rabbitmq(&connection).await;
        println!("{}", connection);
    }

    if !channel.is_open() {
        println!("channel is not open");
        channel = channel_rabbitmq(&connection).await;
    }

    let (ctag, mut messages_rx) = match channel.basic_consume_rx(args.clone()).await {
        Ok(res) => res,
        Err(e) => {
            println!("error {}", e.to_string());
            sleep(Duration::from_secs(10)).await;
            return Box::pin(check_for_new_fingerprints(connection_details)).await;
        }
    };

    if let Some(msg) = messages_rx.recv().await {
        let data = msg.content.unwrap();
        let prop = msg.basic_properties.unwrap();
        let song_name = prop.headers().unwrap().get(&FieldName::try_from("song-name").unwrap()).unwrap();

        let fingerprints: Vec<Fingerprint> = serde_json::from_slice(&data).unwrap();

        let args = BasicAckArguments::new(msg.deliver.unwrap().delivery_tag(), false);
        let _ = channel.basic_ack(args).await;

        return Ok((song_name.to_string(), fingerprints));
    }

    if let Err(e) = channel.basic_cancel(BasicCancelArguments::new(&ctag)).await {
        println!("error {}", e.to_string());
        return Err(Box::new(e));
    };

    Err(Box::new(io::Error::new(
        io::ErrorKind::Other,
        "An error occurred",
    )))
}

async fn connect_to_rabbitmq(connection_details: &RabbitConnect) -> Connection {
    let mut res = Connection::open(
        &OpenConnectionArguments::new(
            &connection_details.host,
            connection_details.port,
            &connection_details.username,
            &connection_details.password,
        )
        .virtual_host(&connection_details.virtual_host),
    )
    .await;

    while res.is_err() {
        println!("trying to connect after error");
        sleep(Duration::from_millis(2000)).await;
        res = Connection::open(
            &OpenConnectionArguments::new(
                &connection_details.host,
                connection_details.port,
                &connection_details.username,
                &connection_details.password,
            )
            .virtual_host(&connection_details.virtual_host),
        )
        .await;
    }

    let connection = res.unwrap();
    connection
        .register_callback(DefaultConnectionCallback)
        .await
        .unwrap();

    connection
}

async fn channel_rabbitmq(connection: &Connection) -> Channel {
    let channel = connection.open_channel(None).await.unwrap();
    channel
        .register_callback(DefaultChannelCallback)
        .await
        .unwrap();

    let q_args = QueueDeclareArguments::default()
        .queue(String::from("added-songs-queue"))
        .durable(true)
        .finish();
    channel.queue_declare(q_args).await.unwrap().unwrap();

    channel
}

async fn send(
    connection_details: &RabbitConnect,
    queue: &str,
    data: &str,
) {
    let mut connection = connect_to_rabbitmq(connection_details).await;
    let mut channel = channel_rabbitmq(&connection).await;


    if !connection.is_open() {
        println!("Connection not open");
        connection = connect_to_rabbitmq(connection_details).await;
        channel = channel_rabbitmq(&connection).await;
        println!("{}", connection);
    }

    if !channel.is_open() {
        println!("channel is not open");
        channel = channel_rabbitmq(&connection).await;
    } else {
        let args = BasicPublishArguments::new("", queue);

        channel
            .basic_publish(
                BasicProperties::default()
                    .with_delivery_mode(DELIVERY_MODE_PERSISTENT)
                    .finish(),
                data.into(),
                args,
            )
            .await
            .unwrap();
    }
}
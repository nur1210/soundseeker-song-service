pub mod db;
mod grpc;
mod prometheus;
mod rabbitmq;

use dotenv::dotenv;
use std::error::Error;

use grpc::start_grpc_server;
use prometheus::start_prometheus_exporter;
use crate::db::pg::PostgresRepository;
use crate::rabbitmq::consume_rabbitmq;

#[tokio::main]
async fn main() -> Result<(), Box<dyn Error>> {
    dotenv().ok();
    let repository = PostgresRepository::open().await?;
    repository.migrate().await?;


    start_prometheus_exporter();
    consume_rabbitmq(&repository).await;
    start_grpc_server(repository).await?;

    Ok(())
}

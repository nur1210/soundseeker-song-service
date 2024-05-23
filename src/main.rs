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
    let rabbitmq_consumer = consume_rabbitmq(&repository);
    let grpc_server = start_grpc_server(repository.clone());

    let (_grpc_result, _rabbitmq_result) = tokio::join!(grpc_server, rabbitmq_consumer);

    Ok(())
}

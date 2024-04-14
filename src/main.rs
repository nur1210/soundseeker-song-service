mod db;

use tonic::{transport::Server, Request, Response, Status, async_trait};
use echo::{echo_service_server::{EchoService as EService, EchoServiceServer},
           EchoRequest, EchoResponse};
use song::song_service_server::SongService as SService;
use db::Repository;
use std::error::Error;
use std::time::Duration;
use crate::db::pg::PostgresRepository;
use crate::song::{AddSongRequest, AddSongResponse, RecognizeSongRequest, RecognizeSongResponse};
use crate::song::song_service_server::SongServiceServer;
use metrics::{counter, describe_counter};
use metrics_exporter_prometheus::PrometheusBuilder;
use metrics_util::MetricKindMask;

pub mod echo {
    tonic::include_proto!("echo");
}

pub mod song {
    tonic::include_proto!("song");
}

#[derive(Debug)]
pub struct SongService {
    repo: PostgresRepository,
}

impl SongService {
    pub async fn new() -> Result<SongService, Box<dyn Error>> {
        let repo = PostgresRepository::open().await?;
        Ok(SongService { repo })
    }
}

#[async_trait]
impl SService for SongService {
    async fn add_song(
        &self,
        request: Request<AddSongRequest>,
    ) -> Result<Response<AddSongResponse>, Status> {
        counter!("grpc_requests_total").increment(1);

        let add_song_request = request.into_inner();
        let hash_array: Vec<usize> = add_song_request.hashes.iter().map(|&x| x as usize).collect();
        match self.repo.index(&add_song_request.song_name, &hash_array).await {
            Ok(id) => {
                let response = AddSongResponse {
                    song_id: id,
                };
                Ok(Response::new(response))
            }
            Err(e) => Err(Status::aborted(e.to_string())),
        }
    }

    async fn recognize_song(
        &self,
        request: Request<RecognizeSongRequest>,
    ) -> Result<Response<RecognizeSongResponse>, Status> {
        counter!("grpc_requests_total").increment(1);

        let add_song_request = request.into_inner();
        let hash_array: Vec<usize> = add_song_request.hashes.iter().map(|&x| x as usize).collect();
        match self.repo.find(&hash_array).await {
            Ok(found) => {
                match found {
                    Some(name) => {
                        let response = RecognizeSongResponse {
                            song_name: name,
                        };
                        Ok(Response::new(response))
                    }
                    None => Err(Status::not_found("Could not recognize the song"))
                }
            }
            Err(e) => Err(Status::aborted(e.to_string()))
        }
    }
}

#[derive(Debug, Default)]
pub struct EchoService {}

#[async_trait]
impl EService for EchoService {
    async fn echo(
        &self,
        request: Request<EchoRequest>,
    ) -> Result<Response<EchoResponse>, Status> {
        counter!("grpc_requests_total").increment(1);
        println!("Got a request: {:?}", request);

        let response = EchoResponse {
            message: format!("Echo: {}", request.into_inner().message),
        };

        Ok(Response::new(response))
    }
}


#[tokio::main]
async fn main() -> Result<(), Box<dyn Error>> {
    start_promeheus_exporter();
    start_grpc_server().await?;

    Ok(())
}

async fn start_grpc_server() -> Result<(), Box<dyn Error>> {
    let grpc_addr = "0.0.0.0:50051".parse()?;
    let echo_service = EchoService::default();
    let song_service = SongService::new().await?;

    Server::builder()
        .add_service(EchoServiceServer::new(echo_service))
        .add_service(SongServiceServer::new(song_service))
        .serve(grpc_addr)
        .await?;
    
    Ok(())
}

fn start_promeheus_exporter() {
    PrometheusBuilder::new()
        .with_http_listener(([0, 0, 0, 0], 9090))
        .idle_timeout(
            MetricKindMask::COUNTER | MetricKindMask::HISTOGRAM | MetricKindMask::GAUGE,
            Some(Duration::from_secs(10)),
        )
        .install()
        .expect("failed to install Prometheus recorder");

    describe_counter!("grpc_requests_total", "grpc requests");
}
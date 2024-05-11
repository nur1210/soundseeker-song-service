use crate::db::pg::{Fingerprint, PostgresRepository};
use crate::db::Repository;
use echo::{
    echo_service_server::{EchoService as EService, EchoServiceServer},
    EchoRequest, EchoResponse,
};
use metrics::{counter, histogram};
use song::song_service_server::SongService as SService;
use song::song_service_server::SongServiceServer;
use song::{RecognizeSongRequest, RecognizeSongResponse};
use std::error::Error;
use std::time::Instant;
use tonic::transport::Server;
use tonic::{async_trait, Request, Response, Status};

pub mod echo {
    tonic::include_proto!("echo");
    pub(crate) const FILE_DESCRIPTOR_SET: &[u8] =
        tonic::include_file_descriptor_set!("echo_descriptor");
}

pub mod song {
    tonic::include_proto!("song");
}

#[derive(Debug)]
pub struct SongService {
    repo: PostgresRepository,
}

impl SongService {
    pub async fn new(repository: PostgresRepository) -> Result<SongService, Box<dyn Error>> {
        let repo = repository;
        Ok(SongService { repo })
    }
}

#[async_trait]
impl SService for SongService {
    async fn recognize_song(
        &self,
        request: Request<RecognizeSongRequest>,
    ) -> Result<Response<RecognizeSongResponse>, Status> {
        let start = Instant::now();
        counter!("grpc_requests_total").increment(1);

        let add_song_request = request.into_inner();
        let fingerprints: Vec<Fingerprint> = add_song_request.fingerprints.iter().map(|x| {
            Fingerprint {
                hash: x.hash.clone(),
                time: x.time as usize,
            }
        })
            .collect();

        match self.repo.find(fingerprints).await {
            Ok(found) => match found {
                Some(name) => {
                    let response = RecognizeSongResponse { song_name: name };
                    histogram!("grpc_request_latency_seconds_bucket").record(start.elapsed());
                    Ok(Response::new(response))
                }
                None => Err(Status::not_found("Could not recognize the song")),
            },
            Err(e) => Err(Status::aborted(e.to_string())),
        }
    }
}

#[derive(Debug, Default)]
pub struct EchoService {}

#[async_trait]
impl EService for EchoService {
    async fn echo(&self, request: Request<EchoRequest>) -> Result<Response<EchoResponse>, Status> {
        let start = Instant::now();
        println!("Got a request: {:?}", request);

        let response = EchoResponse {
            message: format!("Echo: {}", request.into_inner().message),
        };
        counter!("grpc_requests_total").increment(1);
        histogram!("grpc_request_latency_seconds_bucket").record(start.elapsed());
        Ok(Response::new(response))
    }
}

pub async fn start_grpc_server(repository: PostgresRepository) -> Result<(), Box<dyn Error>> {
    let grpc_addr = "0.0.0.0:50051".parse()?;
    let echo_service = EchoService::default();
    let song_service = SongService::new(repository).await?;

    let reflection_service = tonic_reflection::server::Builder::configure()
        .register_encoded_file_descriptor_set(echo::FILE_DESCRIPTOR_SET)
        .build()?;

    Server::builder()
        .add_service(reflection_service)
        .add_service(EchoServiceServer::new(echo_service))
        .add_service(SongServiceServer::new(song_service))
        .serve(grpc_addr)
        .await?;

    Ok(())
}

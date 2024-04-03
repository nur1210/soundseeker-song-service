use tonic::{transport::Server, Request, Response, Status};
use echo::{echo_service_server::EchoService as Service,
           EchoRequest, EchoResponse,
           echo_service_server::EchoServiceServer};

pub mod echo {
    tonic::include_proto!("echo");
}

#[derive(Debug, Default)]
pub struct EchoService {}

#[tonic::async_trait]
impl Service for EchoService {
    async fn echo(
        &self,
        request: Request<EchoRequest>,
    ) -> Result<Response<EchoResponse>, Status> {
        println!("Got a request: {:?}", request);

        let response = EchoResponse {
            message: format!("Echo: {}", request.into_inner().message),
        };

        Ok(Response::new(response))
    }
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let addr = "0.0.0.0:50051".parse()?;
    let echo_service = EchoService::default();

    Server::builder()
        .add_service(EchoServiceServer::new(echo_service))
        .serve(addr)
        .await?;
    Ok(())
}

use env_logger::Env;
use log::{debug, info};
use server::{
    virtual_machine_service_server::{VirtualMachineService, VirtualMachineServiceServer},
    UpsertVirtualMachineRequest, UpsertVirtualMachineResponse,
};
use storage::VirtualMachineScan;
use tonic::{transport::Server, Response};

pub mod storage {
    tonic::include_proto!("storage");
}

pub mod server {
    tonic::include_proto!("sensor");
}

#[derive(Debug, Default)]
struct VMServer {}

#[tonic::async_trait]
impl VirtualMachineService for VMServer {
    #[must_use]
    #[allow(
        elided_named_lifetimes,
        clippy::type_complexity,
        clippy::type_repetition_in_bounds
    )]
    async fn upsert_virtual_machine(
        &self,
        request: tonic::Request<UpsertVirtualMachineRequest>,
    ) -> std::result::Result<tonic::Response<UpsertVirtualMachineResponse>, tonic::Status>
    where
        'life0: 'async_trait,
        Self: 'async_trait,
    {
        debug!("Request: {request:?}");
        let request = request.into_inner();
        if let Some(vm) = request.virtual_machine {
            info!(
                "Got request with {} packages",
                vm.scan
                    .unwrap_or(VirtualMachineScan::default())
                    .components
                    .len()
            );
        }

        let reply = UpsertVirtualMachineResponse { success: true };

        Ok(Response::new(reply))
    }
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    env_logger::Builder::from_env(Env::default().filter_or("MOCK_SERVER_LOGLEVEL", "info")).init();
    info!("Starting mock server...");
    let addr = "127.0.0.1:8080".parse()?;
    let server = VMServer::default();
    Server::builder()
        .add_service(VirtualMachineServiceServer::new(server))
        .serve(addr)
        .await?;

    Ok(())
}

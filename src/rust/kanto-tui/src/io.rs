use super::{
    kanto_api::{self, Result},
    try_best, KantoRequest, KantoResponse, RequestPriority,
    kantui_config
};
use async_priority_channel::{Receiver, Sender};

async fn process_request(
    request: KantoRequest,
    c: &mut kanto_api::cm_rpc::containers_client::ContainersClient<tonic::transport::Channel>,
    response_tx: &Sender<KantoResponse, RequestPriority>,
    config: &kantui_config::AppConfig,
) -> Result<()> {
   match request {
        KantoRequest::ListContainers => {
            let r = kanto_api::list_containers(c).await?;
            try_best(
                response_tx
                    .send(KantoResponse::ListContainers(r), RequestPriority::Low)
                    .await?,
            );
        }
        KantoRequest::_CreateContainer(id, registry) => {
            try_best(kanto_api::create_container(c, &id, &registry).await);
        }
        KantoRequest::StartContainer(id) => {
            try_best(kanto_api::start_container(c, &id).await);
        }
        KantoRequest::StopContainer(id, timeout) => {
            try_best(kanto_api::stop_container(c, &id, timeout).await);
        }
        KantoRequest::RemoveContainer(id) => {
            try_best(kanto_api::remove_container(c, &id, true).await);
        }
        KantoRequest::GetLogs(id) => {
            let logs = match kanto_api::get_container_logs(&id).await {
                Ok(logs) => logs,
                Err(_) => "Could not obtain logs".to_string(),
            };
            try_best(
                response_tx
                    .send(KantoResponse::GetLogs(logs), RequestPriority::Normal)
                    .await,
            );
        }
        KantoRequest::Redeploy => {
            try_best(kanto_api::redeploy_containers(&config.keyconfig.redeploy_command).await);
        }
    }
    Ok(())
}

/// IO Thread
/// Parses requests from the UI thread sent to the request channel and sends the results
/// back to the response channel. This two-channel architecture allows us to set up non-blocking
/// communication between async and sync code.
#[tokio::main]
pub async fn async_io_thread(
    response_tx: Sender<KantoResponse, RequestPriority>,
    request_rx: &mut Receiver<KantoRequest, RequestPriority>,
    config: kantui_config::AppConfig,
) -> kanto_api::Result<()> {
    let mut c = kanto_api::get_connection(&config.socket_path).await?;
    loop {
        if let Ok((request, _)) = request_rx.recv().await {
            process_request(request, &mut c, &response_tx, &config).await?;
        }
    }
}

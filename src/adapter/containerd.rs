use anyhow::{anyhow, Result};
use tokio::net::UnixStream;
use tonic::{Extensions, Request, Response, Status};
use tonic::codegen::ok;
use tonic::metadata::{MetadataMap, MetadataValue};
use tonic::transport::{Channel, Endpoint, Error, Server, Uri};
use tower::service_fn;

use crate::adapter::containerd::services::images::v1::{ListImagesRequest, ListImagesResponse};
use crate::adapter::containerd::services::images::v1::images_client::ImagesClient;

pub struct ContainerdAdapter {
    runtime: tokio::runtime::Runtime,
    tonic_channel: Channel,
}

pub mod services {
    pub mod images {
        pub mod v1 {
            include!("containerd-adapter/containerd.services.images.v1.rs");
        }
    }
}

pub mod types {
    include!("containerd-adapter/containerd.types.rs");
}


impl ContainerdAdapter {
    pub fn new_containerd_adapter() -> Result<ContainerdAdapter> {
        let runtime = tokio::runtime::Builder::new_multi_thread().enable_all().build()?;
        let channel: Result<Channel> = runtime.block_on(async {
            Ok(Endpoint::try_from("http://[::]:50051")?
                .connect_with_connector(service_fn(|_: Uri| {
                    UnixStream::connect("/run/containerd/containerd.sock")
                })).await?)
        });
        Ok(ContainerdAdapter {
            runtime,
            tonic_channel: channel?,
        })
    }


    pub fn image_list(&self) -> Result<Response<ListImagesResponse>> {
        let channel = self.tonic_channel.clone();
        self.runtime.block_on(async {
            let mut request = Request::new(ListImagesRequest {
                filters: vec![]
            });
            let x = request.metadata_mut().append("containerd-namespace", MetadataValue::try_from("k8s.io")?);
            Ok(ImagesClient::new(channel).list(request).await?)
        })
    }
}


#[test]
fn test_image_list() -> Result<()> {
    let adapter = ContainerdAdapter::new_containerd_adapter()?;
    match adapter.image_list() {
        Ok(response) => {}
        Err(err) => {
            println!("{}", err)
        }
    }
    println!("finished");
    Ok(())
}
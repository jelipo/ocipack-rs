use tonic::codegen::ok;
use tonic::Request;
use crate::adapter::containerd::services::images::v1::images_client::ImagesClient;
use crate::adapter::containerd::services::images::v1::ListImagesRequest;

pub struct ContainerdAdapter {
    runtime: tokio::runtime::Runtime,
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

pub fn new_containerd_adapter() -> anyhow::Result<ContainerdAdapter> {
    Ok(ContainerdAdapter {
        runtime: tokio::runtime::Builder::new_multi_thread().enable_all().build()?
    })
}

impl ContainerdAdapter {
    pub fn image_list(&self) {
        let x: anyhow::Result<String> = self.runtime.block_on(async {
            let mut client = ImagesClient::connect("").await?;
            let x1 = client.list(Request::new(ListImagesRequest {
                filters: vec![]
            })).await?;
            Ok("".to_string())
        });
    }
}


pub struct ContainerdAdapter {}

impl ContainerdAdapter {
    pub fn import_image(namespace: &str) {
        let connect1 = containerd::services::images::v1::images_client::ImagesClient::connect("");
    }
}

pub mod containerd {
    pub mod services {
        pub mod images {
            pub mod v1 {
                tonic::include_proto!("containerd.services.images.v1");
            }
        }
    }

    pub mod types {
        tonic::include_proto!("containerd.types");
    }
}


pub mod google {
    pub mod protobuf {
        tonic::include_proto!("google.protobuf");
    }
}

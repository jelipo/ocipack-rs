#[derive(Clone, PartialEq, ::prost::Message)]
pub struct Image {
    /// Name provides a unique name for the image.
    ///
    /// Containerd treats this as the primary identifier.
    #[prost(string, tag="1")]
    pub name: ::prost::alloc::string::String,
    /// Labels provides free form labels for the image. These are runtime only
    /// and do not get inherited into the package image in any way.
    ///
    /// Labels may be updated using the field mask.
    /// The combined size of a key/value pair cannot exceed 4096 bytes.
    #[prost(map="string, string", tag="2")]
    pub labels: ::std::collections::HashMap<::prost::alloc::string::String, ::prost::alloc::string::String>,
    /// Target describes the content entry point of the image.
    #[prost(message, optional, tag="3")]
    pub target: ::core::option::Option<super::super::super::types::Descriptor>,
    /// CreatedAt is the time the image was first created.
    #[prost(message, optional, tag="7")]
    pub created_at: ::core::option::Option<::prost_types::Timestamp>,
    /// UpdatedAt is the last time the image was mutated.
    #[prost(message, optional, tag="8")]
    pub updated_at: ::core::option::Option<::prost_types::Timestamp>,
}
#[derive(Clone, PartialEq, ::prost::Message)]
pub struct GetImageRequest {
    #[prost(string, tag="1")]
    pub name: ::prost::alloc::string::String,
}
#[derive(Clone, PartialEq, ::prost::Message)]
pub struct GetImageResponse {
    #[prost(message, optional, tag="1")]
    pub image: ::core::option::Option<Image>,
}
#[derive(Clone, PartialEq, ::prost::Message)]
pub struct CreateImageRequest {
    #[prost(message, optional, tag="1")]
    pub image: ::core::option::Option<Image>,
}
#[derive(Clone, PartialEq, ::prost::Message)]
pub struct CreateImageResponse {
    #[prost(message, optional, tag="1")]
    pub image: ::core::option::Option<Image>,
}
#[derive(Clone, PartialEq, ::prost::Message)]
pub struct UpdateImageRequest {
    /// Image provides a full or partial image for update.
    ///
    /// The name field must be set or an error will be returned.
    #[prost(message, optional, tag="1")]
    pub image: ::core::option::Option<Image>,
    /// UpdateMask specifies which fields to perform the update on. If empty,
    /// the operation applies to all fields.
    #[prost(message, optional, tag="2")]
    pub update_mask: ::core::option::Option<::prost_types::FieldMask>,
}
#[derive(Clone, PartialEq, ::prost::Message)]
pub struct UpdateImageResponse {
    #[prost(message, optional, tag="1")]
    pub image: ::core::option::Option<Image>,
}
#[derive(Clone, PartialEq, ::prost::Message)]
pub struct ListImagesRequest {
    /// Filters contains one or more filters using the syntax defined in the
    /// containerd filter package.
    ///
    /// The returned result will be those that match any of the provided
    /// filters. Expanded, images that match the following will be
    /// returned:
    ///
    ///    filters\[0\] or filters\[1\] or ... or filters\[n-1\] or filters\[n\]
    ///
    /// If filters is zero-length or nil, all items will be returned.
    #[prost(string, repeated, tag="1")]
    pub filters: ::prost::alloc::vec::Vec<::prost::alloc::string::String>,
}
#[derive(Clone, PartialEq, ::prost::Message)]
pub struct ListImagesResponse {
    #[prost(message, repeated, tag="1")]
    pub images: ::prost::alloc::vec::Vec<Image>,
}
#[derive(Clone, PartialEq, ::prost::Message)]
pub struct DeleteImageRequest {
    #[prost(string, tag="1")]
    pub name: ::prost::alloc::string::String,
    /// Sync indicates that the delete and cleanup should be done
    /// synchronously before returning to the caller
    ///
    /// Default is false
    #[prost(bool, tag="2")]
    pub sync: bool,
}
/// Generated client implementations.
pub mod images_client {
    #![allow(unused_variables, dead_code, missing_docs, clippy::let_unit_value)]
    use tonic::codegen::*;
    use tonic::codegen::http::Uri;
    /// Images is a service that allows one to register images with containerd.
    ///
    /// In containerd, an image is merely the mapping of a name to a content root,
    /// described by a descriptor. The behavior and state of image is purely
    /// dictated by the type of the descriptor.
    ///
    /// From the perspective of this service, these references are mostly shallow,
    /// in that the existence of the required content won't be validated until
    /// required by consuming services.
    ///
    /// As such, this can really be considered a "metadata service".
    #[derive(Debug, Clone)]
    pub struct ImagesClient<T> {
        inner: tonic::client::Grpc<T>,
    }
    impl ImagesClient<tonic::transport::Channel> {
        /// Attempt to create a new client by connecting to a given endpoint.
        pub async fn connect<D>(dst: D) -> Result<Self, tonic::transport::Error>
        where
            D: std::convert::TryInto<tonic::transport::Endpoint>,
            D::Error: Into<StdError>,
        {
            let conn = tonic::transport::Endpoint::new(dst)?.connect().await?;
            Ok(Self::new(conn))
        }
    }
    impl<T> ImagesClient<T>
    where
        T: tonic::client::GrpcService<tonic::body::BoxBody>,
        T::Error: Into<StdError>,
        T::ResponseBody: Body<Data = Bytes> + Send + 'static,
        <T::ResponseBody as Body>::Error: Into<StdError> + Send,
    {
        pub fn new(inner: T) -> Self {
            let inner = tonic::client::Grpc::new(inner);
            Self { inner }
        }
        pub fn with_origin(inner: T, origin: Uri) -> Self {
            let inner = tonic::client::Grpc::with_origin(inner, origin);
            Self { inner }
        }
        pub fn with_interceptor<F>(
            inner: T,
            interceptor: F,
        ) -> ImagesClient<InterceptedService<T, F>>
        where
            F: tonic::service::Interceptor,
            T::ResponseBody: Default,
            T: tonic::codegen::Service<
                http::Request<tonic::body::BoxBody>,
                Response = http::Response<
                    <T as tonic::client::GrpcService<tonic::body::BoxBody>>::ResponseBody,
                >,
            >,
            <T as tonic::codegen::Service<
                http::Request<tonic::body::BoxBody>,
            >>::Error: Into<StdError> + Send + Sync,
        {
            ImagesClient::new(InterceptedService::new(inner, interceptor))
        }
        /// Compress requests with the given encoding.
        ///
        /// This requires the server to support it otherwise it might respond with an
        /// error.
        #[must_use]
        pub fn send_compressed(mut self, encoding: CompressionEncoding) -> Self {
            self.inner = self.inner.send_compressed(encoding);
            self
        }
        /// Enable decompressing responses.
        #[must_use]
        pub fn accept_compressed(mut self, encoding: CompressionEncoding) -> Self {
            self.inner = self.inner.accept_compressed(encoding);
            self
        }
        /// Get returns an image by name.
        pub async fn get(
            &mut self,
            request: impl tonic::IntoRequest<super::GetImageRequest>,
        ) -> Result<tonic::Response<super::GetImageResponse>, tonic::Status> {
            self.inner
                .ready()
                .await
                .map_err(|e| {
                    tonic::Status::new(
                        tonic::Code::Unknown,
                        format!("Service was not ready: {}", e.into()),
                    )
                })?;
            let codec = tonic::codec::ProstCodec::default();
            let path = http::uri::PathAndQuery::from_static(
                "/containerd.services.images.v1.Images/Get",
            );
            self.inner.unary(request.into_request(), path, codec).await
        }
        /// List returns a list of all images known to containerd.
        pub async fn list(
            &mut self,
            request: impl tonic::IntoRequest<super::ListImagesRequest>,
        ) -> Result<tonic::Response<super::ListImagesResponse>, tonic::Status> {
            self.inner
                .ready()
                .await
                .map_err(|e| {
                    tonic::Status::new(
                        tonic::Code::Unknown,
                        format!("Service was not ready: {}", e.into()),
                    )
                })?;
            let codec = tonic::codec::ProstCodec::default();
            let path = http::uri::PathAndQuery::from_static(
                "/containerd.services.images.v1.Images/List",
            );
            self.inner.unary(request.into_request(), path, codec).await
        }
        /// Create an image record in the metadata store.
        ///
        /// The name of the image must be unique.
        pub async fn create(
            &mut self,
            request: impl tonic::IntoRequest<super::CreateImageRequest>,
        ) -> Result<tonic::Response<super::CreateImageResponse>, tonic::Status> {
            self.inner
                .ready()
                .await
                .map_err(|e| {
                    tonic::Status::new(
                        tonic::Code::Unknown,
                        format!("Service was not ready: {}", e.into()),
                    )
                })?;
            let codec = tonic::codec::ProstCodec::default();
            let path = http::uri::PathAndQuery::from_static(
                "/containerd.services.images.v1.Images/Create",
            );
            self.inner.unary(request.into_request(), path, codec).await
        }
        /// Update assigns the name to a given target image based on the provided
        /// image.
        pub async fn update(
            &mut self,
            request: impl tonic::IntoRequest<super::UpdateImageRequest>,
        ) -> Result<tonic::Response<super::UpdateImageResponse>, tonic::Status> {
            self.inner
                .ready()
                .await
                .map_err(|e| {
                    tonic::Status::new(
                        tonic::Code::Unknown,
                        format!("Service was not ready: {}", e.into()),
                    )
                })?;
            let codec = tonic::codec::ProstCodec::default();
            let path = http::uri::PathAndQuery::from_static(
                "/containerd.services.images.v1.Images/Update",
            );
            self.inner.unary(request.into_request(), path, codec).await
        }
        /// Delete deletes the image by name.
        pub async fn delete(
            &mut self,
            request: impl tonic::IntoRequest<super::DeleteImageRequest>,
        ) -> Result<tonic::Response<()>, tonic::Status> {
            self.inner
                .ready()
                .await
                .map_err(|e| {
                    tonic::Status::new(
                        tonic::Code::Unknown,
                        format!("Service was not ready: {}", e.into()),
                    )
                })?;
            let codec = tonic::codec::ProstCodec::default();
            let path = http::uri::PathAndQuery::from_static(
                "/containerd.services.images.v1.Images/Delete",
            );
            self.inner.unary(request.into_request(), path, codec).await
        }
    }
}
/// Generated server implementations.
pub mod images_server {
    #![allow(unused_variables, dead_code, missing_docs, clippy::let_unit_value)]
    use tonic::codegen::*;
    ///Generated trait containing gRPC methods that should be implemented for use with ImagesServer.
    #[async_trait]
    pub trait Images: Send + Sync + 'static {
        /// Get returns an image by name.
        async fn get(
            &self,
            request: tonic::Request<super::GetImageRequest>,
        ) -> Result<tonic::Response<super::GetImageResponse>, tonic::Status>;
        /// List returns a list of all images known to containerd.
        async fn list(
            &self,
            request: tonic::Request<super::ListImagesRequest>,
        ) -> Result<tonic::Response<super::ListImagesResponse>, tonic::Status>;
        /// Create an image record in the metadata store.
        ///
        /// The name of the image must be unique.
        async fn create(
            &self,
            request: tonic::Request<super::CreateImageRequest>,
        ) -> Result<tonic::Response<super::CreateImageResponse>, tonic::Status>;
        /// Update assigns the name to a given target image based on the provided
        /// image.
        async fn update(
            &self,
            request: tonic::Request<super::UpdateImageRequest>,
        ) -> Result<tonic::Response<super::UpdateImageResponse>, tonic::Status>;
        /// Delete deletes the image by name.
        async fn delete(
            &self,
            request: tonic::Request<super::DeleteImageRequest>,
        ) -> Result<tonic::Response<()>, tonic::Status>;
    }
    /// Images is a service that allows one to register images with containerd.
    ///
    /// In containerd, an image is merely the mapping of a name to a content root,
    /// described by a descriptor. The behavior and state of image is purely
    /// dictated by the type of the descriptor.
    ///
    /// From the perspective of this service, these references are mostly shallow,
    /// in that the existence of the required content won't be validated until
    /// required by consuming services.
    ///
    /// As such, this can really be considered a "metadata service".
    #[derive(Debug)]
    pub struct ImagesServer<T: Images> {
        inner: _Inner<T>,
        accept_compression_encodings: EnabledCompressionEncodings,
        send_compression_encodings: EnabledCompressionEncodings,
    }
    struct _Inner<T>(Arc<T>);
    impl<T: Images> ImagesServer<T> {
        pub fn new(inner: T) -> Self {
            Self::from_arc(Arc::new(inner))
        }
        pub fn from_arc(inner: Arc<T>) -> Self {
            let inner = _Inner(inner);
            Self {
                inner,
                accept_compression_encodings: Default::default(),
                send_compression_encodings: Default::default(),
            }
        }
        pub fn with_interceptor<F>(
            inner: T,
            interceptor: F,
        ) -> InterceptedService<Self, F>
        where
            F: tonic::service::Interceptor,
        {
            InterceptedService::new(Self::new(inner), interceptor)
        }
        /// Enable decompressing requests with the given encoding.
        #[must_use]
        pub fn accept_compressed(mut self, encoding: CompressionEncoding) -> Self {
            self.accept_compression_encodings.enable(encoding);
            self
        }
        /// Compress responses with the given encoding, if the client supports it.
        #[must_use]
        pub fn send_compressed(mut self, encoding: CompressionEncoding) -> Self {
            self.send_compression_encodings.enable(encoding);
            self
        }
    }
    impl<T, B> tonic::codegen::Service<http::Request<B>> for ImagesServer<T>
    where
        T: Images,
        B: Body + Send + 'static,
        B::Error: Into<StdError> + Send + 'static,
    {
        type Response = http::Response<tonic::body::BoxBody>;
        type Error = std::convert::Infallible;
        type Future = BoxFuture<Self::Response, Self::Error>;
        fn poll_ready(
            &mut self,
            _cx: &mut Context<'_>,
        ) -> Poll<Result<(), Self::Error>> {
            Poll::Ready(Ok(()))
        }
        fn call(&mut self, req: http::Request<B>) -> Self::Future {
            let inner = self.inner.clone();
            match req.uri().path() {
                "/containerd.services.images.v1.Images/Get" => {
                    #[allow(non_camel_case_types)]
                    struct GetSvc<T: Images>(pub Arc<T>);
                    impl<T: Images> tonic::server::UnaryService<super::GetImageRequest>
                    for GetSvc<T> {
                        type Response = super::GetImageResponse;
                        type Future = BoxFuture<
                            tonic::Response<Self::Response>,
                            tonic::Status,
                        >;
                        fn call(
                            &mut self,
                            request: tonic::Request<super::GetImageRequest>,
                        ) -> Self::Future {
                            let inner = self.0.clone();
                            let fut = async move { (*inner).get(request).await };
                            Box::pin(fut)
                        }
                    }
                    let accept_compression_encodings = self.accept_compression_encodings;
                    let send_compression_encodings = self.send_compression_encodings;
                    let inner = self.inner.clone();
                    let fut = async move {
                        let inner = inner.0;
                        let method = GetSvc(inner);
                        let codec = tonic::codec::ProstCodec::default();
                        let mut grpc = tonic::server::Grpc::new(codec)
                            .apply_compression_config(
                                accept_compression_encodings,
                                send_compression_encodings,
                            );
                        let res = grpc.unary(method, req).await;
                        Ok(res)
                    };
                    Box::pin(fut)
                }
                "/containerd.services.images.v1.Images/List" => {
                    #[allow(non_camel_case_types)]
                    struct ListSvc<T: Images>(pub Arc<T>);
                    impl<T: Images> tonic::server::UnaryService<super::ListImagesRequest>
                    for ListSvc<T> {
                        type Response = super::ListImagesResponse;
                        type Future = BoxFuture<
                            tonic::Response<Self::Response>,
                            tonic::Status,
                        >;
                        fn call(
                            &mut self,
                            request: tonic::Request<super::ListImagesRequest>,
                        ) -> Self::Future {
                            let inner = self.0.clone();
                            let fut = async move { (*inner).list(request).await };
                            Box::pin(fut)
                        }
                    }
                    let accept_compression_encodings = self.accept_compression_encodings;
                    let send_compression_encodings = self.send_compression_encodings;
                    let inner = self.inner.clone();
                    let fut = async move {
                        let inner = inner.0;
                        let method = ListSvc(inner);
                        let codec = tonic::codec::ProstCodec::default();
                        let mut grpc = tonic::server::Grpc::new(codec)
                            .apply_compression_config(
                                accept_compression_encodings,
                                send_compression_encodings,
                            );
                        let res = grpc.unary(method, req).await;
                        Ok(res)
                    };
                    Box::pin(fut)
                }
                "/containerd.services.images.v1.Images/Create" => {
                    #[allow(non_camel_case_types)]
                    struct CreateSvc<T: Images>(pub Arc<T>);
                    impl<
                        T: Images,
                    > tonic::server::UnaryService<super::CreateImageRequest>
                    for CreateSvc<T> {
                        type Response = super::CreateImageResponse;
                        type Future = BoxFuture<
                            tonic::Response<Self::Response>,
                            tonic::Status,
                        >;
                        fn call(
                            &mut self,
                            request: tonic::Request<super::CreateImageRequest>,
                        ) -> Self::Future {
                            let inner = self.0.clone();
                            let fut = async move { (*inner).create(request).await };
                            Box::pin(fut)
                        }
                    }
                    let accept_compression_encodings = self.accept_compression_encodings;
                    let send_compression_encodings = self.send_compression_encodings;
                    let inner = self.inner.clone();
                    let fut = async move {
                        let inner = inner.0;
                        let method = CreateSvc(inner);
                        let codec = tonic::codec::ProstCodec::default();
                        let mut grpc = tonic::server::Grpc::new(codec)
                            .apply_compression_config(
                                accept_compression_encodings,
                                send_compression_encodings,
                            );
                        let res = grpc.unary(method, req).await;
                        Ok(res)
                    };
                    Box::pin(fut)
                }
                "/containerd.services.images.v1.Images/Update" => {
                    #[allow(non_camel_case_types)]
                    struct UpdateSvc<T: Images>(pub Arc<T>);
                    impl<
                        T: Images,
                    > tonic::server::UnaryService<super::UpdateImageRequest>
                    for UpdateSvc<T> {
                        type Response = super::UpdateImageResponse;
                        type Future = BoxFuture<
                            tonic::Response<Self::Response>,
                            tonic::Status,
                        >;
                        fn call(
                            &mut self,
                            request: tonic::Request<super::UpdateImageRequest>,
                        ) -> Self::Future {
                            let inner = self.0.clone();
                            let fut = async move { (*inner).update(request).await };
                            Box::pin(fut)
                        }
                    }
                    let accept_compression_encodings = self.accept_compression_encodings;
                    let send_compression_encodings = self.send_compression_encodings;
                    let inner = self.inner.clone();
                    let fut = async move {
                        let inner = inner.0;
                        let method = UpdateSvc(inner);
                        let codec = tonic::codec::ProstCodec::default();
                        let mut grpc = tonic::server::Grpc::new(codec)
                            .apply_compression_config(
                                accept_compression_encodings,
                                send_compression_encodings,
                            );
                        let res = grpc.unary(method, req).await;
                        Ok(res)
                    };
                    Box::pin(fut)
                }
                "/containerd.services.images.v1.Images/Delete" => {
                    #[allow(non_camel_case_types)]
                    struct DeleteSvc<T: Images>(pub Arc<T>);
                    impl<
                        T: Images,
                    > tonic::server::UnaryService<super::DeleteImageRequest>
                    for DeleteSvc<T> {
                        type Response = ();
                        type Future = BoxFuture<
                            tonic::Response<Self::Response>,
                            tonic::Status,
                        >;
                        fn call(
                            &mut self,
                            request: tonic::Request<super::DeleteImageRequest>,
                        ) -> Self::Future {
                            let inner = self.0.clone();
                            let fut = async move { (*inner).delete(request).await };
                            Box::pin(fut)
                        }
                    }
                    let accept_compression_encodings = self.accept_compression_encodings;
                    let send_compression_encodings = self.send_compression_encodings;
                    let inner = self.inner.clone();
                    let fut = async move {
                        let inner = inner.0;
                        let method = DeleteSvc(inner);
                        let codec = tonic::codec::ProstCodec::default();
                        let mut grpc = tonic::server::Grpc::new(codec)
                            .apply_compression_config(
                                accept_compression_encodings,
                                send_compression_encodings,
                            );
                        let res = grpc.unary(method, req).await;
                        Ok(res)
                    };
                    Box::pin(fut)
                }
                _ => {
                    Box::pin(async move {
                        Ok(
                            http::Response::builder()
                                .status(200)
                                .header("grpc-status", "12")
                                .header("content-type", "application/grpc")
                                .body(empty_body())
                                .unwrap(),
                        )
                    })
                }
            }
        }
    }
    impl<T: Images> Clone for ImagesServer<T> {
        fn clone(&self) -> Self {
            let inner = self.inner.clone();
            Self {
                inner,
                accept_compression_encodings: self.accept_compression_encodings,
                send_compression_encodings: self.send_compression_encodings,
            }
        }
    }
    impl<T: Images> Clone for _Inner<T> {
        fn clone(&self) -> Self {
            Self(self.0.clone())
        }
    }
    impl<T: std::fmt::Debug> std::fmt::Debug for _Inner<T> {
        fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
            write!(f, "{:?}", self.0)
        }
    }
    impl<T: Images> tonic::server::NamedService for ImagesServer<T> {
        const NAME: &'static str = "containerd.services.images.v1.Images";
    }
}

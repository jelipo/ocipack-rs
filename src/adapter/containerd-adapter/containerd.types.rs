/// Descriptor describes a blob in a content store.
///
/// This descriptor can be used to reference content from an
/// oci descriptor found in a manifest.
/// See <https://godoc.org/github.com/opencontainers/image-spec/specs-go/v1#Descriptor>
#[derive(Clone, PartialEq, ::prost::Message)]
pub struct Descriptor {
    #[prost(string, tag="1")]
    pub media_type: ::prost::alloc::string::String,
    #[prost(string, tag="2")]
    pub digest: ::prost::alloc::string::String,
    #[prost(int64, tag="3")]
    pub size: i64,
    #[prost(map="string, string", tag="5")]
    pub annotations: ::std::collections::HashMap<::prost::alloc::string::String, ::prost::alloc::string::String>,
}

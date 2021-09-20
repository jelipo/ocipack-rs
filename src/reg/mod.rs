pub mod client;
pub mod image;
pub mod registry;

pub struct Reference {
    ///Image的名称
    image_name: String,
    /// 可以是TAG或者digest
    reference: String,
}
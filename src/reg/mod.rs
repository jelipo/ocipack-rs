pub mod client;
pub mod image;
pub mod registry;

pub struct Reference {
    ///Image的名称
    pub image_name: String,
    /// 可以是TAG或者digest
    pub reference: String,
}
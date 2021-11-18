pub mod home;
pub mod docker;


pub struct Reference<'a> {
    /// Image的名称
    pub image_name: &'a str,
    /// 可以是TAG或者digest
    pub reference: &'a str,
}

pub enum BlobType {
    Layers,
    Config,
}

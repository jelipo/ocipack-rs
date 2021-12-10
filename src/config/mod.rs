pub mod dockerfile;
pub mod ocifile;

pub struct BaseImage {
    /// registry的host地址
    pub reg_host: String,
    /// image的名称
    pub image_name: String,
    /// 可以是TAG或者digest
    pub reference: String,
}
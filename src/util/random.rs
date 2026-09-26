use rand::distr::{Alphanumeric, SampleString};

/// 生成一个一个随机字符串
pub fn random_str(size: usize) -> String {
    let mut rng = rand::rng();
    Alphanumeric.sample_string(&mut rng, size)
}

use std::iter;

use rand::distr::Alphanumeric;
use rand::Rng;

/// 生成一个一个随机字符串
pub fn random_str(size: usize) -> String {
    let mut rng = rand::rng();
    let random_str: String = iter::repeat(()).map(|()| rng.sample(Alphanumeric)).map(char::from).take(size).collect();
    random_str
}

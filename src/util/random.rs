use std::iter;

use rand::distributions::Alphanumeric;
use rand::{thread_rng, Rng};

/// 生成一个一个随机字符串
pub fn random_str(size: usize) -> String {
    let mut rng = thread_rng();
    let random_str: String = iter::repeat(()).map(|()| rng.sample(Alphanumeric)).map(char::from).take(size).collect();
    random_str
}

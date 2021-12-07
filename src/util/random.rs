use std::iter;
use rand::distributions::Alphanumeric;
use rand::{Rng, thread_rng};

pub fn random_str(size: usize) -> String {
    let mut rng = thread_rng();
    let random_str: String = iter::repeat(())
        .map(|()| rng.sample(Alphanumeric))
        .map(char::from)
        .take(size)
        .collect();
    random_str
}

#[derive(Clone)]
pub struct ProxyInfo {
    pub addr: String,
    pub auth: Option<ProxyAuth>,
}

impl ProxyInfo {
    pub fn new(addr: String, auth: Option<ProxyAuth>) -> ProxyInfo {
        ProxyInfo { addr, auth }
    }
}

#[derive(Clone)]
pub struct ProxyAuth {
    pub username: String,
    pub password: String,
}

impl ProxyAuth {
    pub fn new(username: String, password: String) -> ProxyAuth {
        ProxyAuth { username, password }
    }
}

#[derive(Clone, Debug)]
pub struct Config {
    pub debug: bool,
    pub udp_bind: String,
    pub http_bind: String,
    pub disable_server_density: bool,
}

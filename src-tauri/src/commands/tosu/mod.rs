pub mod listener;

// tosu default address; it can't be changed yet because the sidecar ignores the SERVER_IP/SERVER_PORT envs
pub const TOSU_ADDR: &str = "127.0.0.1:24050";

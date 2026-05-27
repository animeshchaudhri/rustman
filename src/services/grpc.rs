/// Placeholder that will use tonic reflection to discover and invoke services.

pub struct GrpcClient {
    pub endpoint: String,
}

impl GrpcClient {
    pub fn new(endpoint: impl Into<String>) -> Self {
        Self { endpoint: endpoint.into() }
    }
}

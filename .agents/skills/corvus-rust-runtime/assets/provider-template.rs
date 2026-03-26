use async_trait::async_trait;

pub struct ExampleProvider;

#[async_trait]
impl Provider for ExampleProvider {
    async fn send(&self, request: ProviderRequest) -> Result<ProviderResponse, ProviderError> {
        // validate request
        // call external system
        // map failures into typed provider errors
        Err(ProviderError::Unsupported {
            reason: "template implementation required".to_string(),
        })
    }
}

pub fn register_provider(registry: &mut ProviderRegistry) {
    registry.register("example", std::sync::Arc::new(ExampleProvider));
}

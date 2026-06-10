use hickory_resolver::{
    Name, TokioResolver,
    config::{NameServerConfig, ResolverConfig, ResolverOpts},
    lookup::Lookup,
    name_server::TokioConnectionProvider,
};
use hickory_server::proto::{ProtoError, rr::RecordType};
use serde::Deserialize;

#[derive(Debug, Deserialize)]
pub struct ForwarderConfig {
    name_servers: Vec<NameServerConfig>,
}

#[derive(Debug, Deserialize)]
#[serde(from = "ForwarderConfig")]
pub struct Forwarder {
    #[serde(skip)]
    resolver: TokioResolver,
}

impl From<ForwarderConfig> for Forwarder {
    fn from(config: ForwarderConfig) -> Self {
        let mut resolver_config = ResolverConfig::default();
        for ns in config.name_servers {
            resolver_config.add_name_server(ns);
        }

        let resolver = hickory_resolver::Resolver::builder_with_config(
            resolver_config,
            TokioConnectionProvider::default(),
        )
        .with_options(ResolverOpts::default())
        .build();

        Self { resolver }
    }
}

impl Forwarder {
    pub async fn lookup(
        &self,
        name: impl Into<Name>,
        record_type: RecordType,
    ) -> Result<Lookup, ProtoError> {
        let result = self.resolver.lookup(name, record_type).await?;
        Ok(result)
    }
}

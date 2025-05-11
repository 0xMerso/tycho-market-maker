async fn watch(&self, mtx: SharedTychoStreamState) {
    loop {
        let state = mtx.read().await;
        let tokens = state.tokens.clone();
        drop(state);
        // let symbols = tokens.iter().map(|t| t.symbol.clone()).collect::<Vec<_>>();
        tracing::debug!(" 👀 Watching MM activity on {} at address {}", self.config.network, self.config.wallet_public_key);
        let interval = self.config.poll_interval_ms;
        let provider = ProviderBuilder::new().on_http(self.config.rpc.parse().unwrap());
        let mut txs: HashMap<String, bool> = HashMap::new();
        match provider.get_block_number().await {
            Ok(mut previous) => loop {
                tokio::time::sleep(tokio::time::Duration::from_millis(interval / 10)).await; // Wait the chain / RPC indexes new txs ?
                match provider.get_block_number().await {
                    Ok(current) => {
                        if current > previous {
                            tracing::debug!("New block: {}", current);
                            previous = current;
                        } else {
                            continue;
                        }
                    }
                    Err(e) => {
                        tracing::error!("Failed to get latest block: {:?}", e);
                    }
                }
            },
            Err(e) => {
                tracing::error!("Failed to get latest block: {:?}", e);
            }
        }
    }
}

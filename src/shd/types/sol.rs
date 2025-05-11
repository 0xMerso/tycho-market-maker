use alloy::sol;

sol!(
    #[allow(missing_docs)]
    #[sol(rpc)]
    IChainLinkPF,
    "src/shd/utils/abi/Chainlink.json"
);

sol!(
    #[allow(missing_docs)]
    #[sol(rpc)]
    IERC20,
    "src/shd/utils/abi/IERC20.json"
);

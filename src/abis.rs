use ethers::prelude::abigen;

abigen!(
    IStateBridge,
    r#"[
        function propagateRoot() external
    ]"#;

    IWorldId,
    r#"[
        function latestRoot() external returns (uint256)
        error NoRootsSeen()

    ]"#;

    IOptimismStateBridge,
    r#"[
        function opWorldIDAddress() external returns (address)
    ]"#;

    IPolygonStateBridge,
    r#"[
        function fxChildTunnel() external returns (address)
    ]"#;
);

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

use alloy::sol;
use serde::{Deserialize, Serialize};

sol! {
    #[sol(rpc, abi, )]
    interface IWorldIDIdentityManager {
        #[derive(Debug, Serialize, Deserialize)]
        event TreeChanged(uint256 indexed preRoot, uint8 indexed kind, uint256 indexed postRoot);
        function latestRoot() external returns (uint256);
        function registerIdentities(uint256[8] calldata insertionProof, uint256 preRoot, uint32 startIndex, uint256[] calldata identityCommitments, uint256 postRoot) external;
        function deleteIdentities(uint256[8] calldata deletionProof, bytes calldata packedDeletionIndices, uint256 preRoot, uint256 postRoot) external;
    }

    #[sol(rpc)]
    interface IBridgedWorldID {
        #[derive(Serialize, Deserialize)]
        event RootAdded(uint256 root, uint128 timestamp);
        function latestRoot() public view virtual returns (uint256);
        function receiveRoot(uint256 newRoot) external;
    }

    // TODO: Switch over to alloy types.
    //
    // #[sol(rpc, abi)]
    // interface IStateBridge {
    //     #[derive(Serialize, Deserialize)]
    //     function propagateRoot() external;
    // }
    //
    // #[sol(rpc)]
    // interface IWorldId {
    //     #[derive(Serialize, Deserialize)]
    //     function latestRoot() external returns (uint256);
    //     error NoRootsSeen();
    // }
    //
    // #[sol(rpc)]
    // interface IOptimismStateBridge {
    //     #[derive(Serialize, Deserialize)]
    //     function opWorldIDAddress() external returns (address);
    // }
    //
    // #[sol(rpc)]
    // interface IPolygonStateBridge {
    //     #[derive(Serialize, Deserialize)]
    //     function fxChildTunnel() external returns (address);
    // }
}

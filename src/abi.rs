use alloy::sol;
use serde::{Deserialize, Serialize};

sol! {
    #[sol(rpc)]
    interface IWorldIDIdentityManager {
        #[derive(Debug, Serialize, Deserialize)]
        event TreeChanged(uint256 indexed preRoot, uint8 indexed kind, uint256 indexed postRoot);
        function latestRoot() external returns (uint256);
        function registerIdentities(uint256[8] calldata insertionProof, uint256 preRoot, uint32 startIndex, uint256[] calldata identityCommitments, uint256 postRoot) external;
        function deleteIdentities(uint256[8] calldata deletionProof, bytes calldata packedDeletionIndices, uint256 preRoot, uint256 postRoot) external;
    }

    #[sol(rpc)]
    interface IStateBridge {
        function propagateRoot() external;
    }

    #[sol(rpc)]
    interface IBridgedWorldID {
        #[derive(Serialize, Deserialize)]
        event RootAdded(uint256 root, uint128 timestamp);
        function latestRoot() public view virtual returns (uint256);
        function receiveRoot(uint256 newRoot) external;
    }

    #[sol(rpc)]
    contract IOptimismStateBridge {
        function opWorldIDaddress() external returns (address);
    }

    #[sol(rpc)]
    contract IPolygonStateBridge {
        function fxChildTunnel() external returns (address);
    }
}

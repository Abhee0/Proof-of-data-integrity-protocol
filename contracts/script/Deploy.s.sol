// SPDX-License-Identifier: MIT
pragma solidity ^0.8.20;

import {Script, console2} from "forge-std/Script.sol";
import {DataIntegrity} from "../src/DataIntegrity.sol";

/**
 * @title Deploy
 * @notice Foundry deployment script for DataIntegrity.
 *
 * Usage (Sepolia):
 *   forge script script/Deploy.s.sol:Deploy \
 *     --rpc-url $SEPOLIA_RPC_URL \
 *     --private-key $PRIVATE_KEY \
 *     --broadcast \
 *     --verify \
 *     --etherscan-api-key $ETHERSCAN_API_KEY \
 *     -vvvv
 *
 * The `--verify` flag automatically submits the contract source to Etherscan
 * after deployment, enabling public ABI and source inspection.
 */
contract Deploy is Script {
    function run() external returns (DataIntegrity dataIntegrity) {
        // vm.startBroadcast() uses the private key from --private-key CLI arg
        // or the PRIVATE_KEY env var. All calls between start/stop are sent
        // as real transactions on the target network.
        vm.startBroadcast();

        dataIntegrity = new DataIntegrity();

        vm.stopBroadcast();

        // console2.log goes to stdout during --broadcast; useful for CI pipelines
        // to capture the deployed address without parsing JSON artifacts manually.
        console2.log("DataIntegrity deployed at:", address(dataIntegrity));
        console2.log("Deployer:                 ", msg.sender);
        console2.log("Chain ID:                 ", block.chainid);
    }
}

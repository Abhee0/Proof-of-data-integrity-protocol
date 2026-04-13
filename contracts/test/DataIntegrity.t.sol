// SPDX-License-Identifier: MIT
pragma solidity ^0.8.20;

import {Test, console2} from "forge-std/Test.sol";
import {DataIntegrity} from "../src/DataIntegrity.sol";

/**
 * @title DataIntegrityTest
 * @notice Comprehensive test suite for DataIntegrity.sol
 *
 * Run with:   forge test -vvv
 * Gas report: forge test --gas-report
 */
contract DataIntegrityTest is Test {
    DataIntegrity public contract_;

    event ProofStored(
        bytes32 indexed hash,
        address indexed uploader,
        uint256 timestamp,
        string filename
    );

    // Test fixtures
    bytes32 constant HASH_A = keccak256("file_a_content");
    bytes32 constant HASH_B = keccak256("file_b_content");
    string  constant FNAME_A = "document_a.pdf";
    address constant ALICE   = address(0xA11CE);
    address constant BOB     = address(0xB0B);

    // -------------------------------------------------------------------------
    // Setup
    // -------------------------------------------------------------------------

    function setUp() public {
        contract_ = new DataIntegrity();
    }

    // -------------------------------------------------------------------------
    // storeProof — happy path
    // -------------------------------------------------------------------------

    function test_StoreProof_EmitsEvent() public {
        vm.prank(ALICE);

        // expectEmit: check all 3 indexed topics + data
        vm.expectEmit(true, true, false, true);
        emit ProofStored(HASH_A, ALICE, block.timestamp, FNAME_A);

        contract_.storeProof(HASH_A, FNAME_A);
    }

    function test_StoreProof_RecordIsPersisted() public {
        vm.prank(ALICE);
        contract_.storeProof(HASH_A, FNAME_A);

        DataIntegrity.ProofRecord memory rec = contract_.getProofRecord(HASH_A);

        assertEq(rec.timestamp, block.timestamp);
        assertEq(rec.uploader,  ALICE);
        assertEq(rec.filename,  FNAME_A);
    }

    function test_StoreProof_MultipleDistinctHashes() public {
        vm.prank(ALICE);
        contract_.storeProof(HASH_A, "a.txt");

        vm.prank(BOB);
        contract_.storeProof(HASH_B, "b.txt");

        assertTrue(contract_.verifyProof(HASH_A));
        assertTrue(contract_.verifyProof(HASH_B));
    }

    // -------------------------------------------------------------------------
    // storeProof — failure paths
    // -------------------------------------------------------------------------

    function test_StoreProof_RevertOnDuplicate() public {
        vm.prank(ALICE);
        contract_.storeProof(HASH_A, FNAME_A);

        // Second store with same hash should revert
        vm.prank(BOB);
        vm.expectRevert(abi.encodeWithSelector(DataIntegrity.DuplicateProof.selector, HASH_A));
        contract_.storeProof(HASH_A, "impostor.pdf");
    }

    // -------------------------------------------------------------------------
    // verifyProof
    // -------------------------------------------------------------------------

    function test_VerifyProof_ReturnsFalseForUnknown() public view {
        assertFalse(contract_.verifyProof(HASH_A));
    }

    function test_VerifyProof_ReturnsTrueAfterStore() public {
        contract_.storeProof(HASH_A, FNAME_A);
        assertTrue(contract_.verifyProof(HASH_A));
    }

    // -------------------------------------------------------------------------
    // getTimestamp
    // -------------------------------------------------------------------------

    function test_GetTimestamp_ReturnsCorrectValue() public {
        uint256 ts = 1_700_000_000;
        vm.warp(ts); // pin block.timestamp

        contract_.storeProof(HASH_A, FNAME_A);

        assertEq(contract_.getTimestamp(HASH_A), ts);
    }

    function test_GetTimestamp_RevertsForUnknown() public {
        vm.expectRevert(abi.encodeWithSelector(DataIntegrity.ProofNotFound.selector, HASH_A));
        contract_.getTimestamp(HASH_A);
    }

    // -------------------------------------------------------------------------
    // getProofRecord
    // -------------------------------------------------------------------------

    function test_GetProofRecord_RevertsForUnknown() public {
        vm.expectRevert(abi.encodeWithSelector(DataIntegrity.ProofNotFound.selector, HASH_B));
        contract_.getProofRecord(HASH_B);
    }

    // -------------------------------------------------------------------------
    // Fuzz: any bytes32 hash should store and verify cleanly
    // -------------------------------------------------------------------------

    function testFuzz_StoreAndVerify(bytes32 randomHash, string calldata filename) public {
        // Avoid empty hash (reserved as "null" sentinel in some contexts)
        vm.assume(randomHash != bytes32(0));

        contract_.storeProof(randomHash, filename);

        assertTrue(contract_.verifyProof(randomHash));
        assertEq(contract_.getTimestamp(randomHash), block.timestamp);
    }
}

// SPDX-License-Identifier: MIT
pragma solidity ^0.8.20;

import {Test, console} from "forge-std/Test.sol";
import {IPFE} from "ipfe.sol/src/IPFE.sol";
import {Fair} from "../src/Fair.sol";

contract FairTest is Test {
    IPFE public ipfe;
    Fair public fair;
    
    address keeper1 = address(0x1001);
    address keeper2 = address(0x1002);
    address keeper3 = address(0x1003);
    address user = address(0x1004);
    
    uint256 constant N = 21888242871839275222246405745257275088548364400416034343698204186575808495617;
    
    function setUp() public {
        // Deploy IPFE
        ipfe = new IPFE();
        ipfe.initDlogTable(0, 5000);
        
        uint256[2] memory g = [uint256(1), uint256(2)];
        
        // Generate MPK: msk = [2, 3, 5, 7, 11]
        uint256[2][] memory mpk = new uint256[2][](5);
        mpk[0] = ipfe.ecMul(g, 2);
        mpk[1] = ipfe.ecMul(g, 3);
        mpk[2] = ipfe.ecMul(g, 5);
        mpk[3] = ipfe.ecMul(g, 7);
        mpk[4] = ipfe.ecMul(g, 11);
        
        uint256 skSum = 28; // 2+3+5+7+11
        
        fair = new Fair(address(ipfe), mpk, skSum);
        
        // Fund keepers
        vm.deal(keeper1, 100 ether);
        vm.deal(keeper2, 100 ether);
        vm.deal(keeper3, 100 ether);
        vm.deal(user, 100 ether);
    }
    
    function testCreateCDP() public {
        vm.prank(user);
        uint256 cdpId = fair.createCDP{value: 10 ether}(5000e18);
        assertEq(cdpId, 0);
    }
    
    function testStartLiquidationRound() public {
        vm.prank(user);
        fair.createCDP{value: 10 ether}(5000e18);
        
        uint256 roundId = fair.startLiquidationRound(0);
        assertEq(roundId, 0);
    }
    
    function testCommitRevealFlow() public {
        // Create CDP
        vm.prank(user);
        fair.createCDP{value: 10 ether}(5000e18);
        
        // Start round
        uint256 roundId = fair.startLiquidationRound(0);
        
        // Keepers commit
        bytes32 nonce1 = keccak256("nonce1");
        bytes32 nonce2 = keccak256("nonce2");
        
        bytes32 commit1 = keccak256(abi.encodePacked(uint256(0), keeper1, nonce1));
        bytes32 commit2 = keccak256(abi.encodePacked(uint256(0), keeper2, nonce2));
        
        vm.prank(keeper1);
        fair.commit(roundId, commit1);
        
        vm.prank(keeper2);
        fair.commit(roundId, commit2);
        
        // Advance to reveal window
        vm.roll(block.number + 11);
        
        // Keepers reveal
        vm.prank(keeper1);
        fair.reveal(roundId, nonce1, 42);
        
        vm.prank(keeper2);
        fair.reveal(roundId, nonce2, 43);
    }
    
    function testKeeperPoolDistribution() public {
        // Create low-collateral CDP that should be liquidatable
        vm.prank(user);
        fair.createCDP{value: 1 ether}(1500e18); // ~133% ratio
        
        // Start round
        uint256 roundId = fair.startLiquidationRound(0);
        
        // 3 keepers commit
        bytes32 nonce1 = keccak256("nonce1");
        bytes32 nonce2 = keccak256("nonce2");
        bytes32 nonce3 = keccak256("nonce3");
        
        vm.prank(keeper1);
        fair.commit(roundId, keccak256(abi.encodePacked(uint256(0), keeper1, nonce1)));
        
        vm.prank(keeper2);
        fair.commit(roundId, keccak256(abi.encodePacked(uint256(0), keeper2, nonce2)));
        
        vm.prank(keeper3);
        fair.commit(roundId, keccak256(abi.encodePacked(uint256(0), keeper3, nonce3)));
        
        // Advance to reveal window
        vm.roll(block.number + 11);
        
        // All keepers reveal
        vm.prank(keeper1);
        fair.reveal(roundId, nonce1, 42);
        
        vm.prank(keeper2);
        fair.reveal(roundId, nonce2, 43);
        
        vm.prank(keeper3);
        fair.reveal(roundId, nonce3, 44);
        
        // Advance past reveal window
        vm.roll(block.number + 11);
        
        // Record balances before
        uint256 bal1Before = keeper1.balance;
        uint256 bal2Before = keeper2.balance;
        uint256 bal3Before = keeper3.balance;
        
        // Execute liquidation
        // Fund contract: 1 ETH * 2000 price * 13% penalty = 260 USD worth
        // In ETH: ~0.13 ETH profit to distribute
        vm.deal(address(fair), 1 ether);
        fair.executeLiquidation(roundId);
        
        // Check equal distribution
        uint256 bal1After = keeper1.balance;
        uint256 bal2After = keeper2.balance;
        uint256 bal3After = keeper3.balance;
        
        uint256 profit1 = bal1After - bal1Before;
        uint256 profit2 = bal2After - bal2Before;
        uint256 profit3 = bal3After - bal3Before;
        
        // All should get equal share
        assertEq(profit1, profit2, "Keeper 1 and 2 should get equal");
        assertEq(profit2, profit3, "Keeper 2 and 3 should get equal");
        
        console.log("Each keeper received:", profit1);
        console.log("Treasury:", fair.treasury());
    }
    
    function testRedemptionRateUpdate() public {
        // Initial redemption price is 1e18
        assertEq(fair.redemptionPrice(), 1e18);
        
        // Advance time
        vm.warp(block.timestamp + 2 hours);
        
        // Update with market price below target (0.95)
        fair.updateRedemptionRate(0.95e18);
        
        // Rate should be positive to push price up
        assertTrue(fair.redemptionRate() > 0);
    }
}

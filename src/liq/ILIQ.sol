// SPDX-License-Identifier: MIT
pragma solidity ^0.8.20;

import {IERC20} from "forge-std/interfaces/IERC20.sol";
import {IERC3156FlashLender} from "./IERC3156.sol";

interface ILIQ is IERC20, IERC3156FlashLender {
    function usdc() external view returns (address);
    function mint(uint256 amount) external;
    function redeem(uint256 amount) external;
}

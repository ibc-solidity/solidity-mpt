// SPDX-License-Identifier: Apache-2.0
pragma solidity ^0.8.0;

import {ERC20 as AbstractERC20} from "@openzeppelin/contracts/token/ERC20/ERC20.sol";

contract ERC20 is AbstractERC20 {
    constructor(uint256 initialSupply) AbstractERC20("Test", "TST") {
        _mint(msg.sender, initialSupply);
    }
}

// SPDX-License-Identifier: Apache-2.0
pragma solidity ^0.8.0;

import "../src/MPTProof.sol";

contract MPTProofHelper {
    function verifyRLPProof(bytes memory rlpProof, bytes32 rootHash, bytes32 mptKey)
        public
        pure
        returns (bytes memory value)
    {
        return MPTProof.verifyRLPProof(rlpProof, rootHash, mptKey);
    }

    function verify(RLPReader.RLPItem[] memory proof, bytes32 rootHash, bytes memory mptKeyNibbles)
        public
        pure
        returns (bytes memory value)
    {
        return MPTProof.verify(proof, rootHash, mptKeyNibbles);
    }

    function decodeNibbles(bytes memory bz, uint256 offset) public pure returns (bytes memory) {
        return MPTProof.decodeNibbles(bz, offset);
    }
}

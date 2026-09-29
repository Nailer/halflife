// SPDX-License-Identifier: Apache-2.0
pragma solidity ^0.8.24;

/// @notice Hyperlane's recipient interface. Declared locally rather than
/// imported so this repository has no build-time dependency on the monorepo;
/// the signature is fixed by the protocol.
interface IMessageRecipient {
    function handle(uint32 origin, bytes32 sender, bytes calldata message) external payable;
}

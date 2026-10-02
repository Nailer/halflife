// SPDX-License-Identifier: Apache-2.0
pragma solidity ^0.8.24;

import {IMessageRecipient} from "./IMessageRecipient.sol";
import {IInterchainSecurityModule as IIsm, ISpecifiesInterchainSecurityModule} from "./IInterchainSecurityModule.sol";

/// @notice A stand-in for the Hyperlane mailbox, for running the destination
/// against a local chain.
///
/// @dev It is honest about what it does and does not replace. It performs the
/// one check the real mailbox performs before calling a recipient — ask the
/// recipient which ISM it requires, and run that module's `verify` — so the
/// recipient-side contract path is exercised exactly as it would be in
/// production.
///
/// What it does **not** replace is the validator set, the relayer, or the
/// aggregation of real attestations. Those live off-chain and a local chain
/// cannot stand in for them. This exercises the delivery path, not the security
/// of the delivery.
contract MockMailbox {
    event Delivered(address indexed recipient, uint32 origin, bytes32 sender);

    error IsmRejected();

    function deliver(address recipient, uint32 origin, bytes32 sender, bytes calldata message)
        external
    {
        IIsm ism = ISpecifiesInterchainSecurityModule(recipient).interchainSecurityModule();
        if (!ism.verify("", message)) revert IsmRejected();
        IMessageRecipient(recipient).handle(origin, sender, message);
        emit Delivered(recipient, origin, sender);
    }
}

/// @notice An AGGREGATION-type module for local runs.
/// @dev Declares the right type so the destination's guard against `Types.NULL`
/// is genuinely satisfied rather than bypassed.
contract LocalAggregationIsm is IIsm {
    function moduleType() external pure returns (uint8) {
        return uint8(IIsm.Types.AGGREGATION);
    }

    function verify(bytes calldata, bytes calldata) external pure returns (bool) {
        return true;
    }
}

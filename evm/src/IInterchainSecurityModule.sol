// SPDX-License-Identifier: Apache-2.0
pragma solidity ^0.8.24;

/// @notice Hyperlane's ISM interface. Declared locally so this repository has
/// no build-time dependency on the monorepo.
interface IInterchainSecurityModule {
    /// Mirrors Hyperlane's `Types` enum. Only the values this contract reasons
    /// about are named; the rest are deliberately left as raw numbers.
    enum Types {
        UNUSED,
        ROUTING,
        AGGREGATION,
        LEGACY_MULTISIG,
        MERKLE_ROOT_MULTISIG,
        MESSAGE_ID_MULTISIG,
        /// Verifies nothing. The relayer is trusted outright.
        NULL,
        CCIP_READ,
        ARB_L2_TO_L1,
        WEIGHTED_MERKLE_ROOT_MULTISIG,
        WEIGHTED_MESSAGE_ID_MULTISIG,
        OP_L2_TO_L1,
        POLYMER
    }

    function moduleType() external view returns (uint8);

    function verify(bytes calldata metadata, bytes calldata message)
        external
        returns (bool);
}

interface ISpecifiesInterchainSecurityModule {
    function interchainSecurityModule()
        external
        view
        returns (IInterchainSecurityModule);
}

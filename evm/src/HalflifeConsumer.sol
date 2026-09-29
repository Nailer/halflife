// SPDX-License-Identifier: Apache-2.0
pragma solidity ^0.8.24;

import {HalflifeDestinationRegistry as Registry} from "./HalflifeDestinationRegistry.sol";

/// @title Reference consumer on a destination chain
/// @notice A contract that refuses to proceed without current security evidence.
/// @dev Policy lives here, not in Halflife. This contract declares the circuit
///      it depends on, the issuer it trusts and the capability tier it needs.
///      The registry publishes state; it never decides what this contract
///      requires.
contract HalflifeConsumer {
    Registry public immutable registry;
    bytes32 public immutable circuitHash;
    bytes32 public immutable issuer;
    uint8 public immutable minCapability;

    event Proceeded(address indexed caller);

    /// No passport has ever been delivered for this circuit.
    error NoPassport();
    /// Evidence is no longer current. **This is not a finding.**
    error PassportStale();
    /// An accepted issuer reported this circuit should no longer be trusted.
    error PassportInvalid();
    error InsufficientCapability(uint8 have, uint8 need);

    constructor(Registry _registry, bytes32 _circuitHash, bytes32 _issuer, uint8 _minCapability) {
        registry = _registry;
        circuitHash = _circuitHash;
        issuer = _issuer;
        minCapability = _minCapability;
    }

    /// @notice Do the work, but only behind a current passport.
    function submitProof() external {
        _check();
        emit Proceeded(msg.sender);
    }

    /// @notice The decision alone, for inspection without a state change.
    function wouldProceed() external view returns (bool) {
        Registry.Passport memory p = registry.get(circuitHash, issuer);
        if (!p.present || p.capability < minCapability) return false;
        return registry.statusNow(circuitHash, issuer) == Registry.Effective.Valid;
    }

    function _check() internal view {
        Registry.Passport memory p = registry.get(circuitHash, issuer);
        // Absence is not permission. There is no branch where a missing
        // passport means proceed.
        if (!p.present) revert NoPassport();
        if (p.capability < minCapability) {
            revert InsufficientCapability(p.capability, minCapability);
        }

        Registry.Effective e = registry.statusNow(circuitHash, issuer);
        if (e == Registry.Effective.Valid) return;
        // Distinct errors on purpose: stale means the evidence aged out,
        // invalid means someone reported a problem. Collapsing them would make
        // every delivery hiccup look like a vulnerability.
        if (e == Registry.Effective.Stale) revert PassportStale();
        if (e == Registry.Effective.Invalid) revert PassportInvalid();
        revert NoPassport();
    }
}

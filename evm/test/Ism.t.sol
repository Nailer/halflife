// SPDX-License-Identifier: Apache-2.0
pragma solidity ^0.8.24;

import {Test} from "forge-std/Test.sol";
import {HalflifeDestinationRegistry as Registry} from "../src/HalflifeDestinationRegistry.sol";
import {IInterchainSecurityModule as IIsm} from "../src/IInterchainSecurityModule.sol";

contract MockIsm is IIsm {
    uint8 private _t;

    constructor(uint8 t) {
        _t = t;
    }

    function moduleType() external view returns (uint8) {
        return _t;
    }

    function verify(bytes calldata, bytes calldata) external pure returns (bool) {
        return true;
    }
}

/// @notice The weak configuration must be unrepresentable, not merely discouraged.
contract IsmTest is Test {
    address internal constant MAILBOX = address(0xBEEF);
    uint32 internal constant ORIGIN = 1399811149;
    bytes32 internal constant SENDER = bytes32(uint256(0xA11CE));

    MockIsm internal aggregation;
    MockIsm internal nullIsm;

    function setUp() public {
        aggregation = new MockIsm(uint8(IIsm.Types.AGGREGATION));
        nullIsm = new MockIsm(uint8(IIsm.Types.NULL));
    }

    function test_deploysWithAnAggregationIsm() public {
        Registry r = new Registry(MAILBOX, ORIGIN, SENDER, aggregation);
        assertEq(address(r.interchainSecurityModule()), address(aggregation));
        assertEq(r.moduleTypeOf(), uint8(IIsm.Types.AGGREGATION));
    }

    /// address(0) would inherit the mailbox default. That is the configuration
    /// nobody chooses and everybody ends up with.
    function test_cannotDeployWithoutAnIsm() public {
        vm.expectRevert(Registry.IsmRequired.selector);
        new Registry(MAILBOX, ORIGIN, SENDER, IIsm(address(0)));
    }

    /// Types.NULL verifies nothing and trusts the relayer outright.
    function test_cannotDeployWithTheNullIsm() public {
        vm.expectRevert(Registry.NullIsmRefused.selector);
        new Registry(MAILBOX, ORIGIN, SENDER, nullIsm);
    }

    function test_cannotDowngradeToZero() public {
        Registry r = new Registry(MAILBOX, ORIGIN, SENDER, aggregation);
        vm.expectRevert(Registry.IsmRequired.selector);
        r.setIsm(IIsm(address(0)));
    }

    function test_cannotDowngradeToNull() public {
        Registry r = new Registry(MAILBOX, ORIGIN, SENDER, aggregation);
        vm.expectRevert(Registry.NullIsmRefused.selector);
        r.setIsm(nullIsm);
    }

    function test_onlyOwnerMayChangeTheIsm() public {
        Registry r = new Registry(MAILBOX, ORIGIN, SENDER, aggregation);
        MockIsm other = new MockIsm(uint8(IIsm.Types.ROUTING));
        vm.prank(address(0xDEAD));
        vm.expectRevert(Registry.NotOwner.selector);
        r.setIsm(other);
    }

    function test_ownerMayChangeToAnotherNonNullIsm() public {
        Registry r = new Registry(MAILBOX, ORIGIN, SENDER, aggregation);
        MockIsm routing = new MockIsm(uint8(IIsm.Types.ROUTING));
        r.setIsm(routing);
        assertEq(address(r.interchainSecurityModule()), address(routing));
    }
}

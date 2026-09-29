// SPDX-License-Identifier: Apache-2.0
pragma solidity ^0.8.24;

import {Test} from "forge-std/Test.sol";
import {HalflifeDestinationRegistry as Registry} from "../src/HalflifeDestinationRegistry.sol";
import {HalflifeConsumer} from "../src/HalflifeConsumer.sol";

contract DestinationTest is Test {
    Registry internal registry;
    HalflifeConsumer internal consumer;

    address internal constant MAILBOX = address(0xBEEF);
    uint32 internal constant ORIGIN = 1399811149; // Solana mainnet domain
    bytes32 internal constant SENDER = bytes32(uint256(0xA11CE));
    bytes32 internal constant CIRCUIT = bytes32(uint256(0x11));
    bytes32 internal constant ISSUER = bytes32(uint256(0x22));

    function setUp() public {
        registry = new Registry(MAILBOX, ORIGIN, SENDER);
        consumer = new HalflifeConsumer(registry, CIRCUIT, ISSUER, 1);
        vm.warp(1_700_000_000);
    }

    /// Build the canonical 125-byte core, little-endian, exactly as the Rust
    /// encoder produces it.
    function _core(uint64 sequence, uint8 status, int64 issuedAt, int64 expiresAt, uint8 capability)
        internal
        pure
        returns (bytes memory)
    {
        return abi.encodePacked(
            uint8(1),
            CIRCUIT,
            ISSUER,
            _le64(sequence),
            capability,
            status,
            _le64(uint64(issuedAt)),
            _le64(uint64(expiresAt)),
            bytes32(uint256(0x33)),
            _le16(0)
        );
    }

    function _le64(uint64 v) internal pure returns (bytes memory out) {
        out = new bytes(8);
        for (uint256 i = 0; i < 8; i++) {
            out[i] = bytes1(uint8(v >> (8 * i)));
        }
    }

    function _le16(uint16 v) internal pure returns (bytes memory out) {
        out = new bytes(2);
        out[0] = bytes1(uint8(v));
        out[1] = bytes1(uint8(v >> 8));
    }

    function _deliver(bytes memory core) internal {
        vm.prank(MAILBOX);
        registry.handle(ORIGIN, SENDER, core);
    }

    // ---- the encoding actually round-trips -------------------------------

    function test_decodesTheCanonicalCore() public {
        _deliver(_core(7, 1, 1_700_000_000, 1_700_086_400, 4));
        Registry.Passport memory p = registry.get(CIRCUIT, ISSUER);
        assertEq(p.sequence, 7);
        assertEq(p.capability, 4);
        assertEq(p.status, 1);
        assertEq(p.issuedAt, 1_700_000_000);
        assertEq(p.expiresAt, 1_700_086_400);
        assertTrue(p.present);
    }

    // ---- authorization ----------------------------------------------------

    function test_rejectsNonMailboxCaller() public {
        vm.expectRevert(Registry.NotMailbox.selector);
        registry.handle(ORIGIN, SENDER, _core(1, 1, 1_700_000_000, 1_700_086_400, 1));
    }

    function test_rejectsWrongOriginDomain() public {
        vm.prank(MAILBOX);
        vm.expectRevert(abi.encodeWithSelector(Registry.UnexpectedOrigin.selector, uint32(99)));
        registry.handle(99, SENDER, _core(1, 1, 1_700_000_000, 1_700_086_400, 1));
    }

    function test_rejectsWrongSender() public {
        bytes32 impostor = bytes32(uint256(0xBAD));
        vm.prank(MAILBOX);
        vm.expectRevert(abi.encodeWithSelector(Registry.UnexpectedSender.selector, impostor));
        registry.handle(ORIGIN, impostor, _core(1, 1, 1_700_000_000, 1_700_086_400, 1));
    }

    // ---- replay -----------------------------------------------------------

    function test_refusesSupersededSequence() public {
        _deliver(_core(5, 1, 1_700_000_000, 1_700_086_400, 1));
        vm.prank(MAILBOX);
        vm.expectRevert(abi.encodeWithSelector(Registry.SequenceNotIncreasing.selector, uint64(3), uint64(5)));
        registry.handle(ORIGIN, SENDER, _core(3, 1, 1_700_000_000, 1_700_086_400, 1));
    }

    function test_refusesRedeliveryOfTheSameSequence() public {
        _deliver(_core(5, 1, 1_700_000_000, 1_700_086_400, 1));
        vm.prank(MAILBOX);
        vm.expectRevert(abi.encodeWithSelector(Registry.SequenceNotIncreasing.selector, uint64(5), uint64(5)));
        registry.handle(ORIGIN, SENDER, _core(5, 1, 1_700_000_000, 1_700_086_400, 1));
    }

    // ---- the consumer actually halts --------------------------------------

    function test_absenceIsNotPermission() public {
        vm.expectRevert(HalflifeConsumer.NoPassport.selector);
        consumer.submitProof();
        assertFalse(consumer.wouldProceed());
    }

    function test_proceedsOnCurrentPassport() public {
        _deliver(_core(1, 1, 1_700_000_000, 1_700_086_400, 1));
        assertTrue(consumer.wouldProceed());
        consumer.submitProof();
    }

    function test_blocksOnInvalid() public {
        _deliver(_core(1, 2, 1_700_000_000, 1_700_086_400, 1));
        vm.expectRevert(HalflifeConsumer.PassportInvalid.selector);
        consumer.submitProof();
    }

    function test_blocksOnInsufficientCapability() public {
        HalflifeConsumer strict = new HalflifeConsumer(registry, CIRCUIT, ISSUER, 4);
        _deliver(_core(1, 1, 1_700_000_000, 1_700_086_400, 1));
        vm.expectRevert(
            abi.encodeWithSelector(HalflifeConsumer.InsufficientCapability.selector, uint8(1), uint8(4))
        );
        strict.submitProof();
    }

    // ---- THE ONE THAT MATTERS ---------------------------------------------

    /// No invalidation is ever delivered. The relayer simply stops. The stored
    /// passport ages out and the consumer blocks on its own.
    ///
    /// This is why suppressing delivery is useless to an attacker: withholding
    /// a message produces the safe outcome, not the unsafe one.
    function test_blocksWhenNoUpdateEverArrives() public {
        _deliver(_core(1, 1, 1_700_000_000, 1_700_086_400, 1));
        assertTrue(consumer.wouldProceed());

        // Relayer goes away. Nothing is delivered, ever.
        vm.warp(1_700_086_401);

        assertFalse(consumer.wouldProceed());
        vm.expectRevert(HalflifeConsumer.PassportStale.selector);
        consumer.submitProof();

        assertEq(
            uint256(registry.statusNow(CIRCUIT, ISSUER)),
            uint256(Registry.Effective.Stale),
            "must degrade to Stale, never remain Valid"
        );
    }

    /// Invalid outranks the clock: an invalidated passport never reads as
    /// merely stale, however long it sits.
    function test_invalidOutranksTheClock() public {
        _deliver(_core(1, 2, 1_700_000_000, 1_700_086_400, 1));
        vm.warp(1_900_000_000);
        assertEq(
            uint256(registry.statusNow(CIRCUIT, ISSUER)), uint256(Registry.Effective.Invalid)
        );
    }

    // ---- malformed input ---------------------------------------------------

    function test_rejectsWrongLength() public {
        vm.prank(MAILBOX);
        vm.expectRevert(abi.encodeWithSelector(Registry.BadMessageLength.selector, uint256(10)));
        registry.handle(ORIGIN, SENDER, new bytes(10));
    }

    function test_rejectsUnknownStatusByte() public {
        vm.prank(MAILBOX);
        vm.expectRevert(abi.encodeWithSelector(Registry.UnknownStatus.selector, uint8(9)));
        registry.handle(ORIGIN, SENDER, _core(1, 9, 1_700_000_000, 1_700_086_400, 1));
    }
}

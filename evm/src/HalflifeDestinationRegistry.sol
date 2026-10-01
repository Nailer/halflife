// SPDX-License-Identifier: Apache-2.0
pragma solidity ^0.8.24;

import {IMessageRecipient} from "./IMessageRecipient.sol";
import {
    IInterchainSecurityModule,
    ISpecifiesInterchainSecurityModule
} from "./IInterchainSecurityModule.sol";

/// @title Halflife destination registry
/// @notice Receives passport state from the canonical Solana registry over
///         Hyperlane, and serves it to consumers on this chain.
///
/// @dev ## What this contract trusts, stated plainly
///
/// It does **not** verify the issuer's ed25519 signature. EVM has no ed25519
/// precompile, and doing it in Solidity costs more than the check is worth.
/// The chain of trust is therefore:
///
///   issuer signature  →  verified on Solana by the passport program
///                     →  Solana consensus makes that state canonical
///                     →  Hyperlane's ISM attests to what Solana said
///                     →  this contract
///
/// So Hyperlane authenticates *transport*: it establishes that this is what the
/// Solana registry says, not that the issuer's claim is independently true.
/// Solana did that part. A destination chain inherits Solana's verification.
/// Re-verifying ed25519 here is the upgrade path, not a claim we make today.
///
/// ## Why expiry is not enforced on receipt
///
/// `handle` stores whatever arrives. Staleness is resolved at *read* time
/// against the reader's clock, which is what makes suppression useless: if no
/// update ever arrives, the stored passport simply ages out and consumers
/// block. Rejecting expired messages on receipt would instead leave the last
/// good state in place, which is the failure this design exists to prevent.
/// @dev ## Security module, chosen explicitly
///
/// A Hyperlane recipient that returns `address(0)` from
/// `interchainSecurityModule()` inherits the mailbox default. This contract
/// cannot do that: the constructor rejects a zero ISM, and so does the setter.
///
/// It also rejects `Types.NULL` — the module that verifies nothing and trusts
/// the relayer outright. At the time of the $292M KelpDAO loss, 47% of deployed
/// integrations on a comparable system were running a single-verifier
/// configuration. Nobody chose that on purpose; they accepted a default. The
/// defence is to make the weak configuration unrepresentable rather than
/// discouraged, which is why this is a `revert` and not a comment.
///
/// What this contract does *not* do is verify the ISM is strong enough. It can
/// reject the obviously empty choice; it cannot judge a 1-of-1 multisig from a
/// 7-of-10. That remains an operator decision, and `docs/ism-config.md` records
/// the one this deployment uses.
contract HalflifeDestinationRegistry is IMessageRecipient, ISpecifiesInterchainSecurityModule {
    /// Must match `halflife_core::PASSPORT_CORE_LEN`.
    uint256 internal constant CORE_LEN = 125;

    uint8 internal constant STATUS_VALID = 1;
    uint8 internal constant STATUS_INVALID = 2;

    enum Effective {
        Missing,
        Valid,
        Stale,
        Invalid
    }

    struct Passport {
        bytes32 circuitHash;
        bytes32 issuer;
        uint64 sequence;
        uint8 capability;
        uint8 status;
        int64 issuedAt;
        int64 expiresAt;
        bytes32 evidenceHash;
        uint16 advisoryCount;
        /// Distinguishes "never delivered" from "delivered and empty".
        bool present;
    }

    address public immutable mailbox;
    uint32 public immutable originDomain;
    /// The Solana-side registry authorized to speak for this system.
    bytes32 public immutable originSender;

    /// keccak(circuitHash, issuer) => latest passport
    mapping(bytes32 => Passport) private _passports;

    /// The security module the mailbox must satisfy before calling `handle`.
    IInterchainSecurityModule public ism;
    address public owner;

    event PassportReceived(
        bytes32 indexed circuitHash, bytes32 indexed issuer, uint64 sequence, uint8 status
    );

    error NotMailbox();
    error UnexpectedOrigin(uint32 domain);
    error UnexpectedSender(bytes32 sender);
    error BadMessageLength(uint256 length);
    error UnsupportedVersion(uint8 version);
    error UnknownStatus(uint8 status);
    /// Replay, including from a chain that has not seen the newer passport.
    error SequenceNotIncreasing(uint64 incoming, uint64 stored);
    /// Refusing `address(0)` is the whole point: it would silently inherit the
    /// mailbox default.
    error IsmRequired();
    /// `Types.NULL` verifies nothing.
    error NullIsmRefused();
    error NotOwner();

    event IsmChanged(address indexed previous, address indexed current);

    constructor(
        address _mailbox,
        uint32 _originDomain,
        bytes32 _originSender,
        IInterchainSecurityModule _ism
    ) {
        mailbox = _mailbox;
        originDomain = _originDomain;
        originSender = _originSender;
        owner = msg.sender;
        _setIsm(_ism);
    }

    /// @notice Replace the security module.
    /// @dev Same guards as the constructor. There is no path to the default.
    function setIsm(IInterchainSecurityModule _ism) external {
        if (msg.sender != owner) revert NotOwner();
        _setIsm(_ism);
    }

    function interchainSecurityModule()
        external
        view
        override
        returns (IInterchainSecurityModule)
    {
        return ism;
    }

    /// @notice The configured module's self-reported type, for inspection.
    function moduleTypeOf() external view returns (uint8) {
        return ism.moduleType();
    }

    function _setIsm(IInterchainSecurityModule _ism) internal {
        if (address(_ism) == address(0)) revert IsmRequired();
        if (_ism.moduleType() == uint8(IInterchainSecurityModule.Types.NULL)) {
            revert NullIsmRefused();
        }
        emit IsmChanged(address(ism), address(_ism));
        ism = _ism;
    }

    function key(bytes32 circuitHash, bytes32 issuer) public pure returns (bytes32) {
        return keccak256(abi.encodePacked(circuitHash, issuer));
    }

    /// @notice Receive a passport update from the origin registry.
    /// @dev `message` is the canonical 125-byte core, byte-for-byte as signed
    ///      off-chain and stored on Solana. One serialization everywhere: no
    ///      re-encoding, no second format to keep in step.
    function handle(uint32 _origin, bytes32 _sender, bytes calldata message)
        external
        payable
        override
    {
        if (msg.sender != mailbox) revert NotMailbox();
        if (_origin != originDomain) revert UnexpectedOrigin(_origin);
        if (_sender != originSender) revert UnexpectedSender(_sender);
        if (message.length != CORE_LEN) revert BadMessageLength(message.length);

        Passport memory p = _decode(message);

        bytes32 k = key(p.circuitHash, p.issuer);
        Passport storage stored = _passports[k];
        // High-water mark per (circuit, issuer). Equal is rejected too: a
        // re-delivery carries no new information and must not reset anything.
        if (stored.present && p.sequence <= stored.sequence) {
            revert SequenceNotIncreasing(p.sequence, stored.sequence);
        }

        _passports[k] = p;
        emit PassportReceived(p.circuitHash, p.issuer, p.sequence, p.status);
    }

    /// @notice Resolve what a consumer should act on, at `timestamp`.
    /// @dev Ordering is load-bearing. An absent passport is `Missing`, never
    ///      `Valid` — absence is not permission. `Invalid` outranks the clock.
    ///      An expired `Valid` degrades to `Stale` rather than staying valid.
    function statusAt(bytes32 circuitHash, bytes32 issuer, int64 timestamp)
        public
        view
        returns (Effective)
    {
        Passport storage p = _passports[key(circuitHash, issuer)];
        if (!p.present) return Effective.Missing;
        if (p.status == STATUS_INVALID) return Effective.Invalid;
        if (timestamp >= p.expiresAt) return Effective.Stale;
        return Effective.Valid;
    }

    function statusNow(bytes32 circuitHash, bytes32 issuer) external view returns (Effective) {
        return statusAt(circuitHash, issuer, int64(uint64(block.timestamp)));
    }

    function get(bytes32 circuitHash, bytes32 issuer) external view returns (Passport memory) {
        return _passports[key(circuitHash, issuer)];
    }

    /// Fixed-width little-endian, matching `docs/passport-spec.md` exactly.
    function _decode(bytes calldata b) internal pure returns (Passport memory p) {
        uint8 version = uint8(b[0]);
        if (version != 1) revert UnsupportedVersion(version);

        p.circuitHash = bytes32(b[1:33]);
        p.issuer = bytes32(b[33:65]);
        p.sequence = _u64le(b[65:73]);
        p.capability = uint8(b[73]);
        p.status = uint8(b[74]);
        if (p.status != STATUS_VALID && p.status != STATUS_INVALID) {
            revert UnknownStatus(p.status);
        }
        p.issuedAt = int64(_u64le(b[75:83]));
        p.expiresAt = int64(_u64le(b[83:91]));
        p.evidenceHash = bytes32(b[91:123]);
        p.advisoryCount = uint16(_u64le_short(b[123:125]));
        p.present = true;
    }

    /// The core is little-endian; EVM words are big-endian. Reversed explicitly
    /// rather than assumed.
    function _u64le(bytes calldata b) private pure returns (uint64 v) {
        for (uint256 i = 8; i > 0; i--) {
            v = (v << 8) | uint8(b[i - 1]);
        }
    }

    function _u64le_short(bytes calldata b) private pure returns (uint16 v) {
        v = uint16(uint8(b[1])) << 8 | uint16(uint8(b[0]));
    }
}

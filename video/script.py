# Narration for the Halflife demo video.
# Each line: (spoken text for TTS, caption text). "lead"/"tail" are silence
# padding in seconds around the scene's speech; a line's 3rd item is extra
# silence after it (a "hold" so the UI can breathe).

SCENES = [
    dict(id="s01_title", lead=0.8, tail=1.0, lines=[
        ("This is Half-life. A use-by date for cryptographic security.",
         "This is Halflife — a use-by date for cryptographic security."),
    ]),
    dict(id="s02_problem", lead=0.3, tail=0.8, lines=[
        ("In twenty twenty-six, four cryptographic systems that had passed professional review, failed.",
         "In 2026, four cryptographic systems that had passed professional review failed."),
        ("Every flaw was years old. Every one was found with AI assistance.",
         "Every flaw was years old. Every one was found with AI assistance."),
    ]),
    dict(id="s03_insight", lead=0.3, tail=0.6, lines=[
        ("The researcher behind the Zee-cash Orchard bug had already scanned that same code with the previous model generation. It found nothing.",
         "The researcher behind the Zcash Orchard bug had already scanned that same code with the previous model generation. It found nothing."),
        ("The code never changed. The tools got better.",
         "The code never changed. The tools got better.", 0.3),
        ("So an audit is a perishable good. And zero-knowledge bugs are silent. A broken circuit still produces a valid-looking proof.",
         "So an audit is a perishable good. And zero-knowledge bugs are silent: a broken circuit still produces a valid-looking proof."),
    ]),
    dict(id="s04_solution", lead=0.3, tail=0.8, lines=[
        ("Half-life gives a circuit a passport. A signed, hundred-and-twenty-five-byte claim of what was verified, on which build, and when it expires.",
         "Halflife gives a circuit a passport: a signed, 125-byte claim of what was verified, on which build — and when it expires."),
        ("Solana enforces it. Hyperlane carries it across chains. Programs refuse to proceed without one.",
         "Solana enforces it. Hyperlane carries it across chains. Programs refuse to proceed without one."),
    ]),
    dict(id="s05_cli", lead=0.4, tail=0.8, lines=[
        ("Here's the real command line. The vulnerable Orchard build: three advisory hits. Passport signed, invalid.",
         "Here's the real CLI. The vulnerable Orchard build: three advisory hits. Passport signed INVALID."),
        ("The patched build: valid. A consumer accepts one, and blocks the other.",
         "The patched build: VALID. A consumer accepts one — and blocks the other."),
        ("Forge valid onto the bad one, and the signature fails.",
         "Forge VALID onto the bad one, and the signature fails."),
    ]),
    dict(id="s06_room", lead=0.4, tail=0.6, lines=[
        ("This is the live control room. Every number on it comes from real exercises on Solana dev-net.",
         "This is the live control room. Every number on it comes from real exercises on Solana devnet."),
    ]),
    dict(id="s07_impact", lead=0.3, tail=0.4, lines=[
        ("The halo-two gadgets advisory lists its affected crates, and then says: and any dependents thereof.",
         "The halo2_gadgets advisory lists its affected crates, then says: “and any dependents thereof.”", 0.3),
        ("Nothing enumerates them.", "Nothing enumerates them.", 0.9),
        ("This does.", "This does.", 2.2),
    ]),
    dict(id="s08_patched", lead=0.3, tail=0.8, lines=[
        ("Now the patched version. It still reaches three circuits, and it affects none of them.",
         "Now the patched version. It still reaches three circuits — and affects none of them.", 0.4),
        ("Anyone can build an alarm. Showing what a healthy dependency doesn't touch, is what proves it discriminates.",
         "Anyone can build an alarm. Showing what a healthy dependency doesn't touch is what proves it discriminates."),
    ]),
    dict(id="s09_drill", lead=0.3, tail=0.8, lines=[
        ("Now, enforcement. This fire drill was recorded on Solana dev-net, and every signature links to the live explorer.",
         "Now, enforcement. This fire drill was recorded on Solana devnet — every signature links to the live explorer.", 0.3),
        ("A finding lands, the passport is invalidated, and a third-party program refuses to proceed.",
         "A finding lands, the passport is invalidated, and a third-party program refuses to proceed.", 0.3),
        ("That check costs fifteen hundred and twenty compute units. Under one percent of a transaction budget.",
         "That check costs 1,520 compute units — under 1% of a transaction budget."),
    ]),
    dict(id="s10_censor", lead=0.3, tail=1.2, lines=[
        ("Now the one that matters.", "Now the one that matters.", 0.4),
        ("A passport is published, and then, nothing. No invalidation. The relayer simply stops.",
         "A passport is published — and then nothing. No invalidation. The relayer simply stops.", 2.6),
        ("Forty-one seconds later, the consumer blocks anyway. On expiry alone.",
         "41 seconds later, the consumer blocks anyway — on expiry alone.", 0.6),
        ("A passport can't vouch for its own freshness. The reader checks its own clock.",
         "A passport can't vouch for its own freshness. The reader checks its own clock.", 0.2),
        ("So withholding a message produces the safe outcome.",
         "So withholding a message produces the safe outcome."),
    ]),
    dict(id="s11_embargo", lead=0.3, tail=0.8, lines=[
        ("Flip from public, to operator.", "Flip from PUBLIC to OPERATOR.", 0.5),
        ("The same advisory now affects three circuits, not one.",
         "The same advisory now affects three circuits — not one.", 0.3),
        ("These are real crates, under disclosure embargo. In public, they're not redacted. They're absent.",
         "These are real crates under a disclosure embargo. In public they're not redacted — they're absent."),
    ]),
    dict(id="s12_why", lead=0.3, tail=0.8, lines=[
        ("Why Solana? Passports are read-only to consumers, so Sea-level runs every check in parallel. That's how security fits inside a hot path.",
         "Why Solana? Passports are read-only to consumers, so Sealevel runs every check in parallel — that's how security fits inside a hot path."),
    ]),
    dict(id="s13_self", lead=0.3, tail=0.8, lines=[
        ("Finally, Half-life issues a passport for itself, under the same rules.",
         "Finally, Halflife issues a passport for itself, under the same rules.", 0.3),
        ("It comes back invalid. Fifteen advisories in its own closure. We published it anyway.",
         "It comes back INVALID — 15 advisories in its own closure. We published it anyway."),
    ]),
    dict(id="s14_end", lead=0.4, tail=2.6, lines=[
        ("Half-life. No claim, without measurement.", "Halflife. No claim without measurement."),
    ]),
]

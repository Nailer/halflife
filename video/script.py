# Narration for the Halflife demo video (v2).
# Each line: (spoken text for TTS, caption text[, extra hold seconds]).

SCENES = [
    # real landing page: intro stroke, then the hero and its pipeline
    dict(id="s01_title", lead=1.4, tail=0.8, lines=[
        ("This is Half-life. A use-by date for cryptographic security, on Solana.",
         "This is Halflife — a use-by date for cryptographic security, on Solana.", 0.5),
        ("Your program stops trusting a circuit the moment its evidence expires.",
         "Your program stops trusting a circuit the moment its evidence expires."),
    ]),
    dict(id="s02_problem", lead=0.3, tail=0.7, lines=[
        ("In twenty twenty-six, four cryptographic systems that had passed professional review, failed.",
         "In 2026, four cryptographic systems that had passed professional review failed."),
        ("Three had been hiding for four to six years. At least two were found by AI-assisted audits.",
         "Three had been hiding for four to six years. At least two were found by AI-assisted audits."),
    ]),
    dict(id="s03_insight", lead=0.3, tail=0.6, lines=[
        ("Zee-cash Orchard shipped in twenty twenty-two, after review. Four years later, an AI-assisted audit found a counterfeiting bug in it.",
         "Zcash Orchard shipped in 2022, after review. Four years later, an AI-assisted audit found a counterfeiting bug in it."),
        ("The code never changed. The tools got better.",
         "The code never changed. The tools got better.", 0.3),
        ("So an audit is a perishable good. And a broken zero-knowledge circuit fails silently, with valid-looking proofs.",
         "So an audit is a perishable good. And a broken zero-knowledge circuit fails silently, with valid-looking proofs."),
    ]),
    dict(id="s04_solution", lead=0.3, tail=0.7, lines=[
        ("Half-life gives a circuit a passport. A signed, hundred-and-twenty-five-byte claim of what was verified, on which build, and when it expires.",
         "Halflife gives a circuit a passport: a signed, 125-byte claim of what was verified, on which build — and when it expires."),
        ("Solana enforces it. Hyperlane carries it across chains. Programs refuse to proceed without one.",
         "Solana enforces it. Hyperlane carries it across chains. Programs refuse to proceed without one."),
    ]),
    dict(id="s05_room", lead=0.4, tail=0.6, lines=[
        ("This is the live control room. Every number on it comes from real exercises on Solana dev-net.",
         "This is the live control room. Every number on it comes from real exercises on Solana devnet.", 0.3),
        ("Even the Hyperlane message is real, dispatched through the deployed dev-net mailbox.",
         "Even the Hyperlane message is real, dispatched through the deployed devnet mailbox."),
    ]),
    dict(id="s06_impact", lead=0.3, tail=0.4, lines=[
        ("The halo-two gadgets advisory lists its affected crates, and then says: and any dependents thereof.",
         "The halo2_gadgets advisory lists its affected crates, then says: “and any dependents thereof.”", 0.3),
        ("Nothing enumerates them.", "Nothing enumerates them.", 0.9),
        ("This does.", "This does.", 2.0),
    ]),
    dict(id="s07_patched", lead=0.3, tail=0.7, lines=[
        ("The patched version still reaches three circuits, and affects none of them.",
         "The patched version still reaches three circuits — and affects none of them.", 0.3),
        ("Anyone can build an alarm. Showing what a healthy dependency doesn't touch, is what proves it discriminates.",
         "Anyone can build an alarm. Showing what a healthy dependency doesn't touch is what proves it discriminates."),
    ]),
    dict(id="s08_check", lead=0.3, tail=0.8, lines=[
        ("You can run the same check on your own project. Paste a Cargo lock file, and your browser asks O S V directly.",
         "You can run the same check on your own project. Paste a Cargo.lock and your browser asks OSV directly.", 0.6),
        ("The vulnerable Orchard lockfile: would fail.", "The vulnerable Orchard lockfile: WOULD FAIL.", 1.0),
        ("The patched one: would pass.", "The patched one: WOULD PASS."),
    ]),
    dict(id="s09_action", lead=0.3, tail=0.8, lines=[
        ("And it runs in C I. One line of YAML fails a pull request that pulls in a vulnerable dependency.",
         "And it runs in CI: one line of YAML fails a pull request that pulls in a vulnerable dependency."),
    ]),
    dict(id="s10_drill", lead=0.3, tail=0.7, lines=[
        ("Now, enforcement. A fire drill recorded on Solana dev-net. Every signature is on the explorer.",
         "Now, enforcement. A fire drill recorded on Solana devnet — every signature is on the explorer.", 0.3),
        ("A finding lands, the passport is invalidated, and a third-party program refuses to proceed.",
         "A finding lands, the passport is invalidated, and a third-party program refuses to proceed.", 0.3),
        ("That check costs fifteen hundred and twenty compute units, and read-only passports let Sea-level run checks in parallel.",
         "That check costs 1,520 compute units — and read-only passports let Sealevel run checks in parallel."),
    ]),
    dict(id="s11_censor", lead=0.3, tail=1.1, lines=[
        ("Now the one that matters.", "Now the one that matters.", 0.4),
        ("A passport is published, and then, nothing. No invalidation. The relayer simply stops.",
         "A passport is published — and then nothing. No invalidation. The relayer simply stops.", 2.2),
        ("Forty-one seconds later, the consumer blocks anyway. On expiry alone.",
         "41 seconds later, the consumer blocks anyway — on expiry alone.", 0.5),
        ("A passport can't vouch for its own freshness. The reader checks its own clock.",
         "A passport can't vouch for its own freshness. The reader checks its own clock.", 0.2),
        ("So withholding a message produces the safe outcome.",
         "So withholding a message produces the safe outcome."),
    ]),
    dict(id="s12_live", lead=0.3, tail=0.8, lines=[
        ("Don't want to trust our dashboard? Press, read from dev-net.",
         "Don't want to trust our dashboard? Press Read from devnet.", 0.4),
        ("Your browser pulls the real passport from Solana, and decodes the hundred-and-twenty-five bytes itself.",
         "Your browser pulls the real passport from Solana and decodes the 125 bytes itself."),
    ]),
    dict(id="s13_self", lead=0.3, tail=0.8, lines=[
        ("Finally, Half-life issues a passport for itself, under the same rules.",
         "Finally, Halflife issues a passport for itself, under the same rules.", 0.3),
        ("It comes back invalid. Fifteen advisories in its own closure. We published it anyway.",
         "It comes back INVALID — 15 advisories in its own closure. We published it anyway."),
    ]),
    dict(id="s14_end", lead=0.4, tail=2.2, lines=[
        ("Half-life. No claim, without measurement.", "Halflife. No claim without measurement."),
    ]),
]

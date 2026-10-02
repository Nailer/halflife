/** Shapes produced by `halflife export`. The UI never computes state itself. */

export type Status = "VALID" | "STALE" | "INVALID" | "NONE";

export interface Circuit {
  circuitHash: string;
  name: string;
  repository: string;
  commit: string;
  proofSystem: string;
  dependencies: number;
  status: Status;
  capability: number;
  sequence: string;
  expiresAt: string;
  issuer: string;
}

export interface Dependency {
  name: string;
  version: string;
  circuits: number;
  affected: number;
  clean: number;
  advisories: string[];
}

export type EvidenceKind =
  | { type: "SOLANA_TRANSACTION"; signature: string; slot: number }
  | { type: "HYPERLANE_MESSAGE"; message_id: string }
  | { type: "LOCAL"; note: string };

export interface ExerciseEvent {
  seq: number;
  kind: string;
  observed_at: number;
  evidence: EvidenceKind;
  detail: string;
}

export interface Exercise {
  exercise_id: string;
  scenario: "DEPENDENCY_COMPROMISE" | "RELAYER_CENSORSHIP";
  cluster: string;
  started_at: number;
  passport_program: string;
  consumer_program: string;
  circuit_hash: string;
  events: ExerciseEvent[];
}

export interface State {
  generatedAt: string;
  audience: "PUBLIC" | "OPERATOR";
  circuits: Circuit[];
  dependencies: Dependency[];
  exercises: Exercise[];
  deployments: {
    solanaDevnet: {
      passportProgram: string;
      consumerProgram: string;
      hyperlaneMailbox: string;
    };
  };
  measured: {
    consumerCheckCu: number;
    publishCu: number;
    registerIssuerCu: number;
  };
}

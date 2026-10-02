import type { Circuit, Dependency } from "./types";

/** Horizontal lengths used to place the dependency node and the circuit column. */
const DEP_X = 150;
const CIRCUIT_X = 430;
const ROW_H = 46;
const TOP = 34;

/**
 * The blast radius: one dependency, every registered circuit it reaches.
 *
 * Affected and clean are drawn in the same view on purpose. A graph that shows
 * only the damage proves Halflife raises alarms; showing what it *doesn't*
 * touch is what proves it discriminates.
 */
export function BlastRadius({
  dep,
  reached,
}: {
  dep: Dependency | null;
  reached: Circuit[];
}) {
  if (!dep) {
    return (
      <div className="radius">
        <div className="empty">
          <p>
            Pick a dependency to project its blast radius — every registered
            circuit it reaches, and which of them an advisory actually touches.
          </p>
        </div>
      </div>
    );
  }

  const rows = reached.length || 1;
  const h = TOP + rows * ROW_H + (dep.advisories.length ? 46 : 24);
  const midY = TOP + (rows * ROW_H) / 2 - ROW_H / 2;
  const affected = reached.filter((c) => c.status === "INVALID").length;

  return (
    <div className="radius">
      <svg viewBox={`0 0 760 ${h}`} role="img"
        aria-label={`${dep.name} ${dep.version} reaches ${reached.length} circuits, ${affected} affected`}>
        <text x="8" y="14" className="mono" fontSize="9.5" letterSpacing="1.3"
          fill="var(--faint)" style={{ textTransform: "uppercase" }}>
          BLAST RADIUS
        </text>

        {/* the dependency */}
        <g className="node" style={{ animationDelay: "0ms" }}>
          <rect x="8" y={midY - 2} width={DEP_X - 20} height="38" rx="5"
            fill="var(--surface-2)"
            stroke={dep.advisories.length ? "var(--invalid)" : "var(--line-2)"} />
          <text x="20" y={midY + 15} className="mono" fontSize="11.5" fontWeight="700" fill="var(--text)">
            {dep.name.length > 15 ? dep.name.slice(0, 14) + "…" : dep.name}
          </text>
          <text x="20" y={midY + 29} className="mono" fontSize="10"
            fill={dep.advisories.length ? "var(--invalid)" : "var(--dim)"}>
            {dep.version}
          </text>
        </g>

        {/* advisory tag, only when one exists — placed under the dependency
            box so it cannot collide with the section label on a short radius */}
        {dep.advisories.length > 0 && (
          <g className="node" style={{ animationDelay: "80ms" }}>
            <text x="20" y={midY + 50} className="mono" fontSize="9" fill="var(--invalid)">
              {dep.advisories[0]}
            </text>
          </g>
        )}

        {reached.map((c, i) => {
          const y = TOP + i * ROW_H;
          const hit = c.status === "INVALID";
          const colour = hit ? "var(--invalid)" : c.status === "STALE" ? "var(--stale)" : "var(--valid)";
          const x1 = DEP_X - 10;
          const x2 = CIRCUIT_X - 6;
          const y1 = midY + 17;
          const y2 = y + 17;
          const d = `M ${x1} ${y1} C ${x1 + 90} ${y1}, ${x2 - 90} ${y2}, ${x2} ${y2}`;
          const len = Math.hypot(x2 - x1, y2 - y1) + 120;
          return (
            <g key={c.circuitHash}>
              <path
                className="edge"
                d={d}
                fill="none"
                stroke={hit ? "var(--invalid)" : "var(--line-2)"}
                strokeWidth={hit ? 1.8 : 1}
                style={{ ["--len" as string]: len, animationDelay: `${120 + i * 55}ms` }}
              />
              <g className="node" style={{ animationDelay: `${260 + i * 55}ms` }}>
                <rect x={CIRCUIT_X} y={y} width="300" height="34" rx="5"
                  fill="var(--surface-2)" stroke={hit ? "var(--invalid)" : "var(--line)"} />
                <circle cx={CIRCUIT_X + 15} cy={y + 17} r="3.5" fill={colour} />
                <text x={CIRCUIT_X + 28} y={y + 15} fontSize="12" fontWeight="500" fill="var(--text)">
                  {c.name.length > 26 ? c.name.slice(0, 25) + "…" : c.name}
                </text>
                <text x={CIRCUIT_X + 28} y={y + 27} className="mono" fontSize="9.5" fill="var(--faint)">
                  {c.circuitHash.slice(0, 16)}
                </text>
                <text x={CIRCUIT_X + 290} y={y + 21} className="mono" fontSize="9.5"
                  textAnchor="end" fill={colour} fontWeight="700" letterSpacing="0.8">
                  {c.status}
                </text>
              </g>
            </g>
          );
        })}
      </svg>
    </div>
  );
}

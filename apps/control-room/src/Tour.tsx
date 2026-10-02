import { useEffect, useLayoutEffect, useState } from "react";

export interface TourStep {
  /** Matches a `data-tour` attribute in the DOM. */
  target: string;
  title: string;
  body: string;
  /** View to switch to before this step, if the target lives elsewhere. */
  view?: string;
}

const SEEN_KEY = "halflife.tour.v1";

export function hasSeenTour() {
  try {
    return localStorage.getItem(SEEN_KEY) === "1";
  } catch {
    // Private windows and blocked storage throw. Showing the tour again is a
    // far smaller cost than crashing the page over a preference.
    return false;
  }
}

function markSeen() {
  try {
    localStorage.setItem(SEEN_KEY, "1");
  } catch {
    /* nothing to do; the tour simply reappears next visit */
  }
}

interface Box {
  top: number;
  left: number;
  width: number;
  height: number;
}

/**
 * A five-step spotlight tour.
 *
 * The scrim is one enormous box-shadow on the highlight element, so there is a
 * single moving rectangle rather than four stitched panels — which keeps the
 * transition between steps smooth and the DOM honest about what is highlighted.
 */
export function Tour({
  steps,
  onView,
  onDone,
}: {
  steps: TourStep[];
  onView: (v: string) => void;
  onDone: () => void;
}) {
  const [i, setI] = useState(0);
  const [box, setBox] = useState<Box | null>(null);
  const step = steps[i];

  // Switch view first so the target exists before we measure it.
  useEffect(() => {
    if (step?.view) onView(step.view);
  }, [i]);

  useLayoutEffect(() => {
    let frame = 0;
    const timeouts: number[] = [];
    const measure = () => {
      const el = document.querySelector<HTMLElement>(`[data-tour="${step.target}"]`);
      if (!el) {
        setBox(null);
        return;
      }
      el.scrollIntoView({ block: "center", behavior: "smooth" });
      const r = el.getBoundingClientRect();
      const pad = 6;
      setBox({
        top: r.top - pad,
        left: r.left - pad,
        width: r.width + pad * 2,
        height: r.height + pad * 2,
      });
    };
    // The view transition runs 320ms and the scroll is smooth, so measuring
    // early catches the card mid-animation and spotlights the wrong rectangle.
    // Measure once the motion has settled, then again shortly after in case the
    // scroll was still travelling.
    frame = requestAnimationFrame(() => {
      timeouts.push(window.setTimeout(measure, 430), window.setTimeout(measure, 760));
    });
    window.addEventListener("resize", measure);
    return () => {
      cancelAnimationFrame(frame);
      timeouts.forEach(clearTimeout);
      window.removeEventListener("resize", measure);
    };
  }, [i, step]);

  useEffect(() => {
    const key = (e: KeyboardEvent) => {
      if (e.key === "Escape") finish();
      if (e.key === "ArrowRight" || e.key === "Enter") advance();
    };
    window.addEventListener("keydown", key);
    return () => window.removeEventListener("keydown", key);
  }, [i]);

  const finish = () => {
    markSeen();
    onDone();
  };
  const advance = () => (i + 1 < steps.length ? setI(i + 1) : finish());

  if (!step) return null;

  // Place the card below the highlight, flipping above when there is no room.
  const vh = window.innerHeight;
  const below = box ? box.top + box.height + 14 : vh / 2;
  const flip = below > vh - 210;
  const tipTop = box ? (flip ? Math.max(14, box.top - 196) : below) : vh / 2 - 90;
  const tipLeft = box
    ? Math.min(Math.max(14, box.left), window.innerWidth - 344)
    : Math.max(14, window.innerWidth / 2 - 165);

  return (
    <>
      {box ? (
        <div className="tour-hole" style={{ top: box.top, left: box.left, width: box.width, height: box.height }} />
      ) : (
        <div className="tour-hole" style={{ top: -9999, left: -9999, width: 0, height: 0 }} />
      )}

      <div className="tour-tip" style={{ top: tipTop, left: tipLeft }} role="dialog" aria-label={step.title}>
        <div className="step">
          Step {i + 1} of {steps.length}
        </div>
        <h4>{step.title}</h4>
        <p>{step.body}</p>
        <div className="acts">
          <div className="tour-dots" aria-hidden="true">
            {steps.map((_, n) => (
              <i key={n} data-on={n === i} />
            ))}
          </div>
          <div className="spacer" />
          <button className="skip" onClick={finish}>
            Skip
          </button>
          <button className="next" onClick={advance}>
            {i + 1 === steps.length ? "Done" : "Next"}
          </button>
        </div>
      </div>
    </>
  );
}

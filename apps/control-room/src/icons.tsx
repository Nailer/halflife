const s = { width: 16, height: 16, viewBox: "0 0 24 24", fill: "none", stroke: "currentColor", strokeWidth: 2, strokeLinecap: "round" as const, strokeLinejoin: "round" as const };

export const Sun = () => (<svg {...s}><circle cx="12" cy="12" r="4" /><path d="M12 2v2M12 20v2M4.9 4.9l1.4 1.4M17.7 17.7l1.4 1.4M2 12h2M20 12h2M4.9 19.1l1.4-1.4M17.7 6.3l1.4-1.4" /></svg>);
export const Moon = () => (<svg {...s}><path d="M21 12.8A9 9 0 1 1 11.2 3a7 7 0 0 0 9.8 9.8z" /></svg>);
export const Help = () => (<svg {...s}><circle cx="12" cy="12" r="9" /><path d="M9.1 9a3 3 0 1 1 4.2 2.8c-.8.4-1.3 1-1.3 1.9v.3M12 17.5v.5" /></svg>);
export const Search = () => (<svg {...s}><circle cx="11" cy="11" r="7" /><path d="m20 20-3.5-3.5" /></svg>);
export const Play = () => (<svg {...s} fill="currentColor" stroke="none"><path d="M7 4.5v15l12-7.5z" /></svg>);
export const Check = () => (<svg {...s} strokeWidth={2.3}><path d="M20 6 9 17l-5-5" /></svg>);
export const Alert = () => (<svg {...s} strokeWidth={2.3}><circle cx="12" cy="12" r="9" /><path d="M12 7v6M12 16.5v.5" /></svg>);
export const Idle = () => (<svg {...s} strokeWidth={2.2}><circle cx="12" cy="12" r="9" /></svg>);
export const Arrow = () => (<svg {...s}><path d="M5 12h14M13 6l6 6-6 6" /></svg>);

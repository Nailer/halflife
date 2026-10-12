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

// Navigation icons: slightly larger and heavier than the utility set so they
// carry the tab on their own.
const n = { width: 19, height: 19, viewBox: "0 0 24 24", fill: "none", stroke: "currentColor", strokeWidth: 1.9, strokeLinecap: "round" as const, strokeLinejoin: "round" as const };

export const NavOverview = () => (<svg {...n}><rect x="3" y="3" width="7.5" height="7.5" rx="2" /><rect x="13.5" y="3" width="7.5" height="7.5" rx="2" /><rect x="3" y="13.5" width="7.5" height="7.5" rx="2" /><rect x="13.5" y="13.5" width="7.5" height="7.5" rx="2" /></svg>);
export const NavImpact = () => (<svg {...n}><circle cx="12" cy="12" r="2.4" /><circle cx="12" cy="12" r="6" opacity=".65" /><circle cx="12" cy="12" r="9.5" opacity=".35" /></svg>);
export const NavDrills = () => (<svg {...n}><path d="M13 2.5 4.5 13.5H11l-1 8 8.5-11H12z" /></svg>);
export const NavFleet = () => (<svg {...n}><path d="m12 3 9 4.5-9 4.5-9-4.5z" /><path d="m3 12 9 4.5 9-4.5" /><path d="m3 16.5 9 4.5 9-4.5" /></svg>);
export const NavCheck = () => (<svg {...n}><path d="M12 3 4.5 6v5.5c0 4.4 3 8 7.5 9.5 4.5-1.5 7.5-5.1 7.5-9.5V6z" /><path d="m8.8 12.2 2.4 2.4 4.2-4.6" /></svg>);
export const NavAbout = () => (<svg {...n}><circle cx="12" cy="12" r="9.5" /><path d="m15.6 8.4-2 5.2-5.2 2 2-5.2z" /></svg>);
export const NavDocs = () => (<svg {...n}><path d="M4 5.5A2.5 2.5 0 0 1 6.5 3H20v15H6.5A2.5 2.5 0 0 0 4 20.5z" /><path d="M4 20.5A2.5 2.5 0 0 0 6.5 23H20v-5" /><path d="M9 8h7M9 12h5" /></svg>);

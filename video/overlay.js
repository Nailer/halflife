// Overlay toolkit injected into the real control room. Annotates; never alters
// what the app itself renders.
(() => {
  const L = document.createElement('div');
  L.id = 'v-layer';
  document.body.appendChild(L);
  const vig = document.createElement('div'); vig.className = 'v-vig'; L.appendChild(vig);
  const root = document.getElementById('root');
  root.style.transformOrigin = '0 0';
  root.style.transitionProperty = 'transform';
  root.style.transitionTimingFunction = 'cubic-bezier(0.45, 0, 0.2, 1)';
  const cam = { s: 1, tx: 0, ty: 0 };

  const q = (sel) => (typeof sel === 'string' ? document.querySelector(sel) : sel);
  const byText = (sel, txt) => [...document.querySelectorAll(sel)].find((e) => e.textContent.trim().includes(txt));

  // rect of el under the *target* camera, from the actual current transform
  function rect(el) {
    const r = el.getBoundingClientRect();
    const m = new DOMMatrix(getComputedStyle(root).transform === 'none' ? undefined : getComputedStyle(root).transform);
    const s = m.a, tx = m.e, ty = m.f;
    const x = (r.left - tx) / s, y = (r.top - ty) / s, w = r.width / s, h = r.height / s;
    return { left: x * cam.s + cam.tx, top: y * cam.s + cam.ty, width: w * cam.s, height: h * cam.s };
  }

  const OV = {
    q, byText, rect,
    caption(text) {
      L.querySelectorAll('.v-cap').forEach((c) => { c.classList.add('out'); c.dataset.dead = '1'; });
      if (text) {
        const c = document.createElement('div'); c.className = 'v-cap';
        const sp = document.createElement('span'); sp.textContent = text; c.appendChild(sp); L.appendChild(c);
        // shrink-wrap the box to the balanced lines
        const w = Math.max(...[...sp.getClientRects()].map((r) => r.width));
        c.style.width = Math.ceil(w + 46) + 'px';
      }
    },
    reap() { L.querySelectorAll('[data-dead]').forEach((e) => { if (e.getAnimations().every((a) => a.playState === 'finished' || a.currentTime >= 340)) e.remove(); }); },
    chapter(n, t, s) {
      const d = document.createElement('div'); d.className = 'v-chap';
      d.innerHTML = `<span class="n">${n}</span><span class="t">${t}</span>${s ? `<span class="s">${s}</span>` : ''}`;
      L.appendChild(d);
    },
    fadeIn() { const d = document.createElement('div'); d.className = 'v-fade'; L.appendChild(d); },
    cursor(x, y) {
      let c = L.querySelector('.v-cursor');
      if (!c) {
        c = document.createElement('div'); c.className = 'v-cursor';
        c.innerHTML = '<svg width="26" height="26" viewBox="0 0 26 26"><path d="M4 3 L4 21 L9 16.5 L12.6 24 L15.6 22.6 L12 15.3 L18.6 15.3 Z" fill="#fff" stroke="#0b0c0e" stroke-width="1.4" stroke-linejoin="round"/></svg>';
        L.appendChild(c);
      }
      c.style.transitionDuration = '0s'; c.style.transform = `translate(${x - 4}px, ${y - 3}px)`;
      return c;
    },
    move(sel, dur = 0.9, dx = 0, dy = 0) {
      const el = q(sel); const r = rect(el);
      const c = L.querySelector('.v-cursor') || OV.cursor(r.left + r.width / 2 + 120, r.top + r.height / 2 + 160);
      // force a style flush so the transition starts from the old position
      c.getBoundingClientRect();
      c.style.transitionDuration = dur + 's';
      c.style.transform = `translate(${r.left + r.width / 2 + dx - 4}px, ${r.top + r.height / 2 + dy - 3}px)`;
    },
    press() {
      const c = L.querySelector('.v-cursor'); if (!c) return;
      c.classList.remove('press'); void c.offsetWidth; c.classList.add('press');
      const m = new DOMMatrix(getComputedStyle(c).transform);
      const rp = document.createElement('div'); rp.className = 'v-ripple';
      rp.style.left = m.e + 4 + 'px'; rp.style.top = m.f + 3 + 'px'; L.appendChild(rp);
    },
    click(sel) { OV.press(); q(sel).click(); },
    ring(sel, { pad = 8, color, id = 'r' } = {}) {
      const r = rect(q(sel)); const d = document.createElement('div'); d.className = 'v-ring'; d.dataset.id = id;
      if (color) d.style.setProperty('--c', color);
      Object.assign(d.style, { left: r.left - pad + 'px', top: r.top - pad + 'px', width: r.width + pad * 2 + 'px', height: r.height + pad * 2 + 'px' });
      L.appendChild(d);
    },
    unring(id = 'r') { L.querySelectorAll(`.v-ring[data-id="${id}"]`).forEach((e) => e.classList.add('out')); },
    callout({ k, v, d, color, x, y, id = 'c', anchor, side = 'right', gap = 18 }) {
      const el = document.createElement('div'); el.className = 'v-call'; el.dataset.id = id;
      if (color) el.style.setProperty('--c', color);
      el.innerHTML = `<div class="k">${k}</div>${v ? `<div class="v">${v}</div>` : ''}${d ? `<div class="d">${d}</div>` : ''}`;
      L.appendChild(el);
      if (anchor) {
        const r = rect(q(anchor)); const b = el.getBoundingClientRect();
        if (side === 'right') { x = r.left + r.width + gap; y = r.top + r.height / 2 - b.height / 2; }
        if (side === 'left') { x = r.left - gap - b.width; y = r.top + r.height / 2 - b.height / 2; }
        if (side === 'below') { x = r.left + r.width / 2 - b.width / 2; y = r.top + r.height + gap; }
        if (side === 'above') { x = r.left + r.width / 2 - b.width / 2; y = r.top - gap - b.height; }
      }
      el.style.left = x + 'px'; el.style.top = y + 'px';
    },
    uncall(id = 'c') { L.querySelectorAll(`.v-call[data-id="${id}"]`).forEach((e) => e.classList.add('out')); },
    banner(k, v, color) {
      const dim = document.createElement('div'); dim.className = 'v-dim'; L.insertBefore(dim, vig.nextSibling);
      const b = document.createElement('div'); b.className = 'v-banner'; if (color) b.style.setProperty('--c', color);
      b.innerHTML = `<div class="k">${k}</div><div class="v">${v}</div>`; L.appendChild(b);
    },
    camera(sel, s = 1, dur = 1.4, { ox = 0, oy = 0 } = {}) {
      const vw = innerWidth, vh = innerHeight, docH = Math.max(vh, root.scrollHeight);
      let tx = 0, ty = 0;
      if (sel && s !== 1) {
        const el = q(sel); const r0 = el.getBoundingClientRect();
        const m = new DOMMatrix(getComputedStyle(root).transform === 'none' ? undefined : getComputedStyle(root).transform);
        const cx = (r0.left - m.e) / m.a + r0.width / m.a / 2 + ox, cy = (r0.top - m.f) / m.a + r0.height / m.a / 2 + oy;
        tx = vw / 2 - cx * s; ty = vh / 2 - cy * s;
        tx = Math.min(0, Math.max(vw - vw * s, tx)); ty = Math.min(0, Math.max(vh - docH * s, ty));
      }
      Object.assign(cam, { s, tx, ty });
      root.getBoundingClientRect();
      root.style.transitionDuration = dur + 's';
      root.style.transform = `translate(${tx}px, ${ty}px) scale(${s})`;
    },
  };
  window.OV = OV;
})();

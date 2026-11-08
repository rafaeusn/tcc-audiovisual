(() => {
  const root = document.querySelector('.frame');
  const out = { w: innerWidth, h: innerHeight, overflowX: false, clipped: [], small: [], lowContrast: [], dots: 0, sizes: {}, radii: {} };
  const app = document.querySelector('.app');
  out.overflowX = app.scrollWidth > app.clientWidth + 1 || document.documentElement.scrollWidth > innerWidth + 1;
  const vis = (e) => {
    const b = e.getBoundingClientRect();
    if (b.width < 1 || b.height < 1 || b.bottom < 0 || b.top > innerHeight || b.right < 0 || b.left > innerWidth) return false;
    for (let n = e; n && n.nodeType === 1; n = n.parentElement) { const cs = getComputedStyle(n); if (cs.visibility === 'hidden' || cs.display === 'none' || +cs.opacity < 0.05) return false; }
    return true;
  };
  const opac = (e) => { let o = 1; for (let n = e; n && n.nodeType === 1; n = n.parentElement) o *= +getComputedStyle(n).opacity; return o; };
  const parse = (c) => { const m = c && c.match(/rgba?\(([^)]+)\)/); if (!m) return null; const p = m[1].split(/[ ,/]+/).filter(Boolean).map(Number); return { r: p[0], g: p[1], b: p[2], a: p.length > 3 ? p[3] : 1 }; };
  const lin = (v) => { v /= 255; return v <= 0.04045 ? v / 12.92 : Math.pow((v + 0.055) / 1.055, 2.4); };
  const L = (c) => 0.2126 * lin(c.r) + 0.7152 * lin(c.g) + 0.0722 * lin(c.b);
  const mix = (top, base) => ({ r: top.r * top.a + base.r * (1 - top.a), g: top.g * top.a + base.g * (1 - top.a), b: top.b * top.a + base.b * (1 - top.a) });
  const bgOf = (e) => {
    const stack = [];
    for (let n = e; n && n.nodeType === 1; n = n.parentElement) {
      if (n.matches('.hero .tile-in') && !e.closest('.sheet')) return null;
      if (n.matches('.mini, .fs-ui')) return null;
      const cs = getComputedStyle(n);
      if (cs.backgroundImage && cs.backgroundImage !== 'none' && !cs.backgroundImage.includes('gradient')) return null;
      const c = parse(cs.backgroundColor);
      if (c && c.a > 0) { stack.push(c); if (c.a >= 0.99) break; }
    }
    let base = parse(getComputedStyle(root).backgroundColor) || { r: 0, g: 0, b: 0, a: 1 };
    for (let i = stack.length - 1; i >= 0; i--) base = Object.assign(mix(stack[i], base), { a: 1 });
    return base;
  };
  const label = (e) => (e.getAttribute('aria-label') || e.textContent || '').trim().replace(/\s+/g, ' ').slice(0, 34);
  document.querySelectorAll('.frame *').forEach((e) => {
    if (!vis(e)) return;
    const cs = getComputedStyle(e);
    const own = Array.from(e.childNodes).some((n) => n.nodeType === 3 && n.nodeValue.trim());
    if (own) {
      const fs = parseFloat(cs.fontSize);
      out.sizes[fs] = (out.sizes[fs] || 0) + 1;
      if (/·/.test(e.textContent)) out.dots++;
      if (e.scrollWidth > e.clientWidth + 1 && cs.overflow !== 'visible' && cs.textOverflow !== 'ellipsis') out.clipped.push(label(e));
      const fg = parse(cs.color), bg = bgOf(e);
      if (fg && bg) {
        const o = opac(e) * fg.a;
        const c = mix({ r: fg.r, g: fg.g, b: fg.b, a: o }, bg);
        const l1 = L(c), l2 = L(bg), ratio = (Math.max(l1, l2) + 0.05) / (Math.min(l1, l2) + 0.05);
        const large = fs >= 24 || (fs >= 18.66 && +cs.fontWeight >= 700);
        const disabled = !!e.closest('button:disabled, [aria-disabled="true"]');
        if (!disabled && ratio < (large ? 3 : 4.5)) out.lowContrast.push(label(e) + ' ' + ratio.toFixed(2));
      }
    }
    const r = cs.borderTopLeftRadius;
    if (r && r !== '0px' && !e.matches('svg *, i')) out.radii[r] = (out.radii[r] || 0) + 1;
  });
  document.querySelectorAll('.frame button, .frame [role=slider], .frame input:not([type=range]):not([type=color]), .frame a').forEach((e) => {
    if (!vis(e) || e.matches('.scrim, .drawer-scrim')) return;
    const b = e.getBoundingClientRect();
    if (b.width < 24 || b.height < 24) out.small.push(label(e) + ' ' + Math.round(b.width) + 'x' + Math.round(b.height));
  });
  out.clipped = Array.from(new Set(out.clipped));
  out.lowContrast = Array.from(new Set(out.lowContrast));
  out.small = Array.from(new Set(out.small));
  return out;
})()

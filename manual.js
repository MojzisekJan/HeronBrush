// The manual's contents, language switcher and search. The page reads in full without it.
(function () {
  var toc = document.getElementById('toc');
  var toggle = document.querySelector('.toc-toggle');
  var current = document.querySelector('.toc-current');
  var links = Array.prototype.slice.call(toc.querySelectorAll('.toc-list a'));
  var byId = {};
  links.forEach(function (a) { byId[a.getAttribute('href').slice(1)] = a; });
  var headings = Array.prototype.slice.call(document.querySelectorAll('.manual-body [id^="s"]'));
  var chapters = Array.prototype.slice.call(toc.querySelectorAll('.toc-list .chapter'));
  toc.querySelector('.toc-list').classList.add('folding');

  // The section in view: the last heading above a line a little below the header.
  var active = null;
  function sectionOf(id) { return id.split('-').slice(0, 2).join('-'); }
  function update() {
    var line = 120, here = headings[0];
    for (var i = 0; i < headings.length; i++) {
      if (headings[i].getBoundingClientRect().top <= line) here = headings[i]; else break;
    }
    var id = here ? sectionOf(here.id) : null;
    if (id === active) return;
    if (active && byId[active]) byId[active].classList.remove('current');
    active = id;
    var link = byId[id];
    if (link) {
      link.classList.add('current');
      var chapter = link.closest('.chapter');
      chapters.forEach(function (c) { c.classList.toggle('open', c === chapter); });
      current.textContent = link.textContent;
      var r = link.getBoundingClientRect(), t = toc.getBoundingClientRect();
      if (getComputedStyle(toc).position === 'sticky' && (r.top < t.top + 90 || r.bottom > t.bottom)) {
        toc.scrollTop += r.top - t.top - t.height / 2;
      }
    }
  }
  var ticking = false;
  window.addEventListener('scroll', function () {
    if (!ticking) { ticking = true; requestAnimationFrame(function () { ticking = false; update(); }); }
  }, { passive: true });
  update();

  // The drawer on narrow windows.
  function setOpen(open) {
    toc.classList.toggle('open', open);
    toggle.setAttribute('aria-expanded', open ? 'true' : 'false');
  }
  toggle.addEventListener('click', function (e) { e.stopPropagation(); setOpen(!toc.classList.contains('open')); });
  toc.addEventListener('click', function (e) { if (e.target.closest('a')) setOpen(false); });
  document.addEventListener('click', function (e) { if (!toc.contains(e.target) && e.target !== toggle) setOpen(false); });
  document.addEventListener('keydown', function (e) { if (e.key === 'Escape' && toc.classList.contains('open')) { setOpen(false); toggle.focus(); } });

  // Another language at the same section: every translation numbers its sections alike.
  document.getElementById('lang').addEventListener('change', function () {
    location.href = this.value + (active ? '#' + active : '');
  });

  // Search over this language's sections, loaded the first time it is used.
  var input = document.getElementById('search'), results = document.getElementById('results'), index = null;
  function load() {
    if (index) return Promise.resolve(index);
    return fetch('search.json').then(function (r) { return r.json(); }).then(function (d) { index = d; return d; });
  }
  function escape(s) { return s.replace(/[&<>"]/g, function (c) { return { '&': '&amp;', '<': '&lt;', '>': '&gt;', '"': '&quot;' }[c]; }); }
  function search() {
    var q = input.value.trim().toLowerCase();
    if (!q) { results.hidden = true; results.innerHTML = ''; return; }
    load().then(function (items) {
      var terms = q.split(/\s+/), found = [];
      items.forEach(function (it) {
        var t = it.t.toLowerCase(), x = it.x.toLowerCase(), score = 0;
        for (var i = 0; i < terms.length; i++) {
          var inT = t.indexOf(terms[i]) >= 0, inX = x.indexOf(terms[i]) >= 0;
          if (!inT && !inX) return;
          score += inT ? 3 : 1;
        }
        found.push([score, it]);
      });
      found.sort(function (a, b) { return b[0] - a[0]; });
      if (!found.length) { results.innerHTML = '<li class="none">' + escape(results.dataset.nothing) + '</li>'; results.hidden = false; return; }
      results.innerHTML = found.slice(0, 12).map(function (f) {
        var it = f[1], x = it.x, at = x.toLowerCase().indexOf(terms[0]);
        var start = Math.max(0, at - 30), snippet = (start ? '…' : '') + x.slice(start, start + 90) + '…';
        var marked = escape(snippet).replace(new RegExp('(' + terms[0].replace(/[.*+?^${}()|[\]\\]/g, '\\$&') + ')', 'ig'), '<mark>$1</mark>');
        return '<li><a href="#' + it.id + '">' + escape(it.t) + '<small>' + marked + '</small></a></li>';
      }).join('');
      results.hidden = false;
    });
  }
  input.addEventListener('input', search);
  input.addEventListener('focus', load);
  results.addEventListener('click', function (e) { if (e.target.closest('a')) { input.value = ''; results.hidden = true; } });
  input.addEventListener('keydown', function (e) {
    if (e.key === 'Escape') { input.value = ''; results.hidden = true; }
    if (e.key === 'Enter') { var first = results.querySelector('a'); if (first) { first.click(); location.hash = first.getAttribute('href'); } }
  });
})();

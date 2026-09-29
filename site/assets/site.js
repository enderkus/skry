// Install tabs and copy buttons. No tracking, no external requests.
(function () {
  var tabs = document.querySelectorAll("[data-tab]");
  function show(name) {
    tabs.forEach(function (t) { t.classList.toggle("active", t.dataset.tab === name); });
    document.querySelectorAll("[data-pane]").forEach(function (p) { p.hidden = p.dataset.pane !== name; });
  }
  tabs.forEach(function (t) { t.addEventListener("click", function () { show(t.dataset.tab); }); });
  if (/Win/i.test(navigator.platform || navigator.userAgent)) show("windows");

  document.querySelectorAll("[data-copy]").forEach(function (b) {
    b.addEventListener("click", function () {
      var text = document.getElementById(b.dataset.copy).textContent;
      var done = function () {
        var old = b.textContent;
        b.textContent = b.dataset.done || "Copied";
        setTimeout(function () { b.textContent = old; }, 1600);
      };
      if (navigator.clipboard) navigator.clipboard.writeText(text).then(done);
      else {
        var r = document.createRange(); r.selectNode(document.getElementById(b.dataset.copy));
        var s = getSelection(); s.removeAllRanges(); s.addRange(r); document.execCommand("copy"); s.removeAllRanges(); done();
      }
    });
  });
})();

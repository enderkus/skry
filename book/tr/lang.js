// Adds a link to the same page in the other language, and a link back to
// the project home page, to the top bar of every documentation page.
(function () {
  var cur = document.documentElement.lang === "tr" ? "tr" : "en";
  var other = cur === "tr" ? "en" : "tr";
  var bar = document.querySelector(".right-buttons");
  if (!bar) return;
  var path = location.pathname;
  var re = /\/docs\/(en|tr)\//;
  var target = re.test(path) ? path.replace(re, "/docs/" + other + "/") : "../" + other + "/";
  var lang = document.createElement("a");
  lang.href = target + location.hash;
  lang.className = "skry-top-link";
  lang.textContent = cur === "tr" ? "English" : "Türkçe";
  lang.title = cur === "tr" ? "Read this page in English" : "Bu sayfayı Türkçe oku";
  var home = document.createElement("a");
  home.href = cur === "tr" ? "/skry/tr/" : "/skry/";
  home.className = "skry-top-link";
  home.textContent = cur === "tr" ? "Ana sayfa" : "Home";
  bar.insertBefore(lang, bar.firstChild);
  bar.insertBefore(home, bar.firstChild);
})();

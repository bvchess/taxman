// The tone of the player's and the taxman's colors.  The site's own is lightest; ?tone=deep or ?tone=light shows
// a page in another, from the tones css/site.css defines.  A classic script in the head, so the page is drawn in its
// tone from the start.
(function () {
  var tone = new URLSearchParams(location.search).get('tone');
  if (tone) document.documentElement.setAttribute('data-tone', tone);
})();

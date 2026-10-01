// Runs before the page renders (blocking, in <head>): marks that JavaScript is on, so
// the rendered stills of the 3D world stay hidden until story.js decides between the
// world and the stills. Without JavaScript (e.g. Tor Browser "Safest") the stills show.
document.documentElement.classList.add('js');

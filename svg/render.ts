// Renders the Mermaid drawing on the standard input as an SVG on the standard
// output, with Pele. The first argument is the CSS of the theme, which goes
// in a <style> element at the start of the SVG. `garimpo diagrama` runs this.

import { text } from "node:stream/consumers";
import { render } from "pele";

const css = process.argv[2] ?? "";
const { svg } = render(await text(process.stdin));
// The end of the opening <svg …> tag.
const start = svg.indexOf(">") + 1;
process.stdout.write(`${svg.slice(0, start)}<style>${css}</style>${svg.slice(start)}\n`);

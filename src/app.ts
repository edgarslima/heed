/** Renders the empty application shell into the given root element. */
export function mountApp(root: HTMLElement): void {
  const main = document.createElement("main");
  main.className = "app";
  root.replaceChildren(main);
}

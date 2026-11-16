const root = document.querySelector<HTMLElement>("[data-docs-root]");
if (root) {
  const navigation = root.querySelector<HTMLDetailsElement>(".docs-navigation");
  const compact = matchMedia("(max-width: 900px)");
  if (navigation && compact.matches) navigation.open = false;
  compact.addEventListener("change", event => {
    if (navigation) navigation.open = !event.matches;
  });
  const input = root.querySelector<HTMLInputElement>("#docs-search");
  const status = root.querySelector<HTMLElement>(".docs-search-status");
  const normalize = (value: string) => value.normalize("NFD").replace(/[\u0300-\u036f]/g, "").toLowerCase();
  const items = Array.from(root.querySelectorAll<HTMLElement>("[data-doc-item]")).map(element => ({
    element, text: normalize(element.dataset.search ?? ""), slug: element.dataset.docSlug!,
  }));
  const filter = () => {
    const terms = normalize(input?.value.trim() ?? "").split(/\s+/).filter(Boolean);
    const visible = new Set<string>();
    for (const item of items) {
      item.element.hidden = !terms.every(term => item.text.includes(term));
      if (!item.element.hidden) visible.add(item.slug);
    }
    for (const group of root.querySelectorAll<HTMLElement>("[data-doc-group]")) {
      group.hidden = !group.querySelector("[data-doc-item]:not([hidden])");
    }
    for (const empty of root.querySelectorAll<HTMLElement>(".docs-empty")) empty.hidden = visible.size > 0;
    if (status) status.textContent = terms.length ? `${visible.size} ${status.dataset.resultsLabel}` : "";
  };
  root.querySelector<HTMLElement>(".docs-search")?.removeAttribute("hidden");
  input?.addEventListener("input", filter);
  // Browsers can restore an input's value on back/forward navigation.
  window.addEventListener("pageshow", filter);
}

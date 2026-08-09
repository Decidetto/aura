/**
 * Adds the WAI-ARIA tablist keyboard pattern without replacing native buttons.
 * @param {Iterable<HTMLElement>} tabs
 * @param {(tab: HTMLElement) => void} activate
 */
export function bindTabKeyboardNavigation(tabs, activate) {
  const items = Array.from(tabs);
  const isVisible = (tab) => tab.offsetParent !== null;

  const wrap = (index, step) => (index + step + items.length) % items.length;
  const nextVisible = (fromIndex, step) => {
    for (let count = 0; count < items.length; count += 1) {
      const index = wrap(fromIndex, count * step);
      if (isVisible(items[index])) return items[index];
    }
    return null;
  };

  items.forEach((tab, index) => {
    tab.addEventListener("keydown", (event) => {
      if (!isVisible(tab)) return;
      let target = null;

      if (event.key === "ArrowDown" || event.key === "ArrowRight") {
        target = nextVisible(index, 1);
      } else if (event.key === "ArrowUp" || event.key === "ArrowLeft") {
        target = nextVisible(index, -1);
      } else if (event.key === "Home") {
        target = nextVisible(0, 1);
      } else if (event.key === "End") {
        target = nextVisible(items.length - 1, -1);
      } else {
        return;
      }

      if (!target) return;
      event.preventDefault();
      target.focus();
      activate(target);
    });
  });
}

/**
 * Keeps screen-reader pronunciation aligned with the selected UI language.
 * @param {string} language
 */
export function setDocumentLanguage(language) {
  const supported = new Set(["ru", "en", "de", "es", "fr", "it", "zh", "pt", "tr"]);
  document.documentElement.lang = supported.has(language) ? language : "ru";
}

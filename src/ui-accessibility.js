/**
 * Adds the WAI-ARIA tablist keyboard pattern without replacing native buttons.
 * @param {Iterable<HTMLElement>} tabs
 * @param {(tab: HTMLElement) => void} activate
 */
export function bindTabKeyboardNavigation(tabs, activate) {
  const items = Array.from(tabs);

  items.forEach((tab, index) => {
    tab.addEventListener("keydown", (event) => {
      let nextIndex = index;

      if (event.key === "ArrowDown" || event.key === "ArrowRight") {
        nextIndex = (index + 1) % items.length;
      } else if (event.key === "ArrowUp" || event.key === "ArrowLeft") {
        nextIndex = (index - 1 + items.length) % items.length;
      } else if (event.key === "Home") {
        nextIndex = 0;
      } else if (event.key === "End") {
        nextIndex = items.length - 1;
      } else {
        return;
      }

      event.preventDefault();
      const nextTab = items[nextIndex];
      nextTab.focus();
      activate(nextTab);
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

// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.

(() => {
    const darkThemes = ['ayu', 'navy', 'coal'];
    const lightThemes = ['light', 'rust'];
    const sizeClasses = ['diagram-wide', 'diagram-medium', 'diagram-compact'];

    const classList = document.getElementsByTagName('html')[0].classList;

    let lastThemeWasLight = true;
    for (const cssClass of classList) {
        if (darkThemes.includes(cssClass)) {
            lastThemeWasLight = false;
            break;
        }
    }

    const theme = lastThemeWasLight ? 'default' : 'dark';
    mermaid.initialize({
        startOnLoad: true,
        theme,
        flowchart: { useMaxWidth: false },
        sequence: { useMaxWidth: false },
        er: { useMaxWidth: false },
        themeVariables: {
            fontSize: '18px'
        }
    });

    function classifyDiagram(block) {
        const svg = block.querySelector('svg');
        if (!svg) {
            return false;
        }

        let diagramClass = 'diagram-medium';
        const viewBox = svg.getAttribute('viewBox');
        if (viewBox) {
            const parts = viewBox.trim().split(/\s+/).map(Number);
            if (parts.length === 4 && Number.isFinite(parts[2]) && Number.isFinite(parts[3]) && parts[3] > 0) {
                const aspect = parts[2] / parts[3];
                if (aspect >= 2.1) {
                    diagramClass = 'diagram-wide';
                } else if (aspect < 1.2) {
                    diagramClass = 'diagram-compact';
                }
            }
        }

        for (const cls of sizeClasses) {
            block.classList.remove(cls);
        }
        block.classList.add(diagramClass);
        return true;
    }

    function normalizeDiagrams() {
        const blocks = document.querySelectorAll('.mermaid');
        for (const block of blocks) {
            classifyDiagram(block);
        }
    }

    let normalizeTimer = null;
    function scheduleNormalize() {
        if (normalizeTimer !== null) {
            clearTimeout(normalizeTimer);
        }
        normalizeTimer = setTimeout(() => {
            normalizeDiagrams();
            normalizeTimer = null;
        }, 40);
    }

    window.addEventListener('load', () => {
        let pass = 0;
        const maxPasses = 18;
        const tick = () => {
            normalizeDiagrams();
            pass += 1;
            if (pass < maxPasses) {
                requestAnimationFrame(tick);
            }
        };
        requestAnimationFrame(tick);
    });

    const observer = new MutationObserver(() => {
        scheduleNormalize();
    });
    observer.observe(document.body, { childList: true, subtree: true });

    // Simplest way to make mermaid re-render the diagrams in the new theme is via refreshing the page

    for (const darkTheme of darkThemes) {
        document.getElementById(darkTheme).addEventListener('click', () => {
            if (lastThemeWasLight) {
                window.location.reload();
            }
        });
    }

    for (const lightTheme of lightThemes) {
        document.getElementById(lightTheme).addEventListener('click', () => {
            if (!lastThemeWasLight) {
                window.location.reload();
            }
        });
    }
})();

(function () {
  'use strict';

  var COPIED_MS = 1500;

  function fallbackCopy(text) {
    var area = document.createElement('textarea');
    area.value = text;
    area.setAttribute('readonly', '');
    area.style.position = 'fixed';
    area.style.top = '0';
    area.style.opacity = '0';
    document.body.appendChild(area);
    area.select();
    var ok = false;
    try {
      ok = document.execCommand('copy');
    } catch (e) {
      ok = false;
    }
    document.body.removeChild(area);
    return ok ? Promise.resolve() : Promise.reject(new Error('copy failed'));
  }

  function writeClipboard(text) {
    if (navigator.clipboard && navigator.clipboard.writeText) {
      return navigator.clipboard.writeText(text).catch(function () {
        return fallbackCopy(text);
      });
    }
    return fallbackCopy(text);
  }

  function markCopied(button) {
    button.setAttribute('data-copied', 'true');
    window.setTimeout(function () {
      button.removeAttribute('data-copied');
    }, COPIED_MS);
  }

  function bindCopy(button, getText) {
    button.addEventListener('click', function () {
      writeClipboard(getText()).then(function () {
        markCopied(button);
      }, function () {});
    });
  }

  function setupCopyButtons() {
    document.querySelectorAll('.copy-button[data-copy]').forEach(function (button) {
      bindCopy(button, function () {
        return button.getAttribute('data-copy') || '';
      });
    });
  }

  // コードブロックの帯に出す言語名。生のHTMLは html として示す。
  function codeLanguage(block) {
    if (block.classList.contains('raw-html')) {
      return 'html';
    }
    var code = block.querySelector('code');
    var match = code && /(?:^|\s)language-(\S+)/.exec(code.className);
    return match ? match[1] : '';
  }

  function setupCodeCopy() {
    document.querySelectorAll('pre.code-block').forEach(function (block) {
      var language = codeLanguage(block);
      if (language) {
        block.setAttribute('data-lang', language);
      }
      var button = document.createElement('button');
      button.type = 'button';
      button.className = 'copy-button code-copy';
      button.setAttribute('aria-label', 'コードをコピー');
      bindCopy(button, function () {
        var code = block.querySelector('code');
        return (code || block).textContent || '';
      });
      block.appendChild(button);
    });
  }

  function cssValue(name) {
    return window.getComputedStyle(document.documentElement).getPropertyValue(name).trim();
  }

  // base テーマのときに、図の色をページの色へ合わせる。
  function mermaidThemeVariables() {
    var bg = cssValue('--mb-color-bg');
    var surface = cssValue('--mb-color-surface');
    var text = cssValue('--mb-color-text');
    var muted = cssValue('--mb-color-muted');
    var border = cssValue('--mb-color-border');
    var accent = cssValue('--mb-color-accent');
    var accentSoft = cssValue('--mb-color-accent-soft');
    return {
      fontFamily: cssValue('--mb-font-body'),
      background: bg,
      primaryColor: accentSoft,
      primaryTextColor: text,
      primaryBorderColor: accent,
      secondaryColor: surface,
      secondaryTextColor: text,
      secondaryBorderColor: border,
      tertiaryColor: bg,
      tertiaryTextColor: text,
      tertiaryBorderColor: border,
      lineColor: muted,
      textColor: text,
      edgeLabelBackground: bg,
      clusterBkg: surface,
      clusterBorder: border,
      noteBkgColor: surface,
      noteTextColor: text,
      noteBorderColor: border
    };
  }

  function setupMermaid() {
    var blocks = document.querySelectorAll('pre.mermaid');
    if (!window.mermaid || blocks.length === 0) {
      return;
    }
    try {
      var theme = document.body.dataset.mermaidTheme;
      var options = {
        startOnLoad: false,
        securityLevel: 'strict',
        theme: theme,
        fontFamily: cssValue('--mb-font-body'),
        // 日本語のラベルが数文字で折り返されないよう、箱の中の幅を広げる。
        flowchart: { wrappingWidth: 240 }
      };
      if (theme === 'base') {
        options.themeVariables = mermaidThemeVariables();
      }
      window.mermaid.initialize(options);
      var result = window.mermaid.run({ querySelector: 'pre.mermaid' });
      if (result && typeof result.catch === 'function') {
        result.catch(function () {});
      }
    } catch (e) {
      // 図の描画に失敗しても、ほかの部分は動かす。
    }
  }

  setupCopyButtons();
  setupCodeCopy();
  setupMermaid();
})();

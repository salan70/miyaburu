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

  function setupCodeCopy() {
    document.querySelectorAll('pre.code-block').forEach(function (block) {
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

  function setupMermaid() {
    var blocks = document.querySelectorAll('pre.mermaid');
    if (!window.mermaid || blocks.length === 0) {
      return;
    }
    try {
      window.mermaid.initialize({
        startOnLoad: false,
        securityLevel: 'strict',
        theme: document.body.dataset.mermaidTheme
      });
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
